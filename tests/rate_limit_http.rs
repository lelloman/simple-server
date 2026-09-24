#![cfg(feature = "rate-limit")]
use http::{Request, Response, request::Parts};
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use simple_server::rate_limit::*;
use std::{
    convert::Infallible,
    num::{NonZeroU32, NonZeroUsize},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tower::{Layer, Service, ServiceExt};
fn limiter() -> KeyedLimiter<()> {
    KeyedLimiter::new(
        Quota::replenishing(Duration::from_secs(3600), NonZeroU32::new(10).unwrap()).unwrap(),
        StoreConfig::bounded(NonZeroUsize::new(1).unwrap(), Duration::ZERO),
    )
}
fn policy(l: KeyedLimiter<()>) -> AsyncPolicy<Parts, Rejection> {
    AsyncPolicy::from_sync(Policy::new(move |_: &Parts| {
        l.acquire(
            (),
            NonZeroU32::new(1).unwrap(),
            Some(NonZeroUsize::new(1).unwrap()),
        )
    }))
}

#[tokio::test]
async fn streams_without_buffering_and_holds_slot_until_eof_or_drop() {
    let l = limiter();
    // A data frame followed by a pending stream must continue holding the permit.
    let inner = tower::service_fn(|_: Request<()>| async {
        let stream = futures_util::stream::iter(vec![Ok::<_, Infallible>(http_body::Frame::data(
            axum_bytes(b"chunk"),
        ))]);
        use futures_util::StreamExt;
        let stream = stream.chain(futures_util::stream::pending());
        Ok::<_, Infallible>(Response::new(StreamBody::new(stream).boxed_unsync()))
    });
    let reject = |r, _: &Parts| rejection_response::<Full<_>>(r).map(BodyExt::boxed_unsync);
    let service = RateLimitLayer::new(policy(l.clone()), reject).layer(inner);
    let mut response = service.clone().oneshot(Request::new(())).await.unwrap();
    assert_eq!(l.stats().active, 1);
    let frame = response.body_mut().frame().await.unwrap().unwrap();
    assert_eq!(frame.into_data().unwrap().as_ref(), b"chunk");
    assert_eq!(l.stats().active, 1);
    let denied = service.clone().oneshot(Request::new(())).await.unwrap();
    assert_eq!(denied.status(), 429);
    assert_eq!(l.stats().active, 1);
    drop(response);
    assert_eq!(l.stats().active, 0);
    drop(service.oneshot(Request::new(())).await.unwrap());
    assert_eq!(l.stats().active, 0);
}
// Use the bytes type already exported through http-body-util's Full inference.
fn axum_bytes(bytes: &'static [u8]) -> bytes::Bytes {
    bytes::Bytes::from_static(bytes)
}

#[tokio::test]
async fn release_on_end_of_stream_error_and_empty_body() {
    let l = limiter();
    let inner = tower::service_fn(|_: Request<()>| async {
        Ok::<_, Infallible>(Response::new(Full::new(axum_bytes(b"end"))))
    });
    let service = RateLimitLayer::new(policy(l.clone()), |e, _: &Parts| {
        rejection_response::<Full<_>>(e)
    })
    .layer(inner);
    let mut response = service.oneshot(Request::new(())).await.unwrap();
    assert_eq!(l.stats().active, 1);
    response.body_mut().frame().await.unwrap().unwrap();
    assert_eq!(l.stats().active, 0); // retain response object; EOF is sufficient
    let inner = tower::service_fn(|_: Request<()>| async {
        Ok::<_, Infallible>(Response::new(Empty::<bytes::Bytes>::new()))
    });
    let service = RateLimitLayer::new(policy(l.clone()), |_: Rejection, _: &Parts| {
        Response::new(Empty::<bytes::Bytes>::new())
    })
    .layer(inner);
    let response = service.oneshot(Request::new(())).await.unwrap();
    assert_eq!(l.stats().active, 0);
    drop(response);

    let inner = tower::service_fn(|_: Request<()>| async {
        Ok::<_, Infallible>(Response::new(StreamBody::new(futures_util::stream::iter(
            vec![Err::<http_body::Frame<bytes::Bytes>, _>("broken")],
        ))))
    });
    let service = RateLimitLayer::new(policy(l.clone()), |_: Rejection, _: &Parts| {
        Response::new(StreamBody::new(futures_util::stream::iter(vec![])))
    })
    .layer(inner);
    let mut response = service.oneshot(Request::new(())).await.unwrap();
    assert!(response.body_mut().frame().await.unwrap().is_err());
    assert_eq!(l.stats().active, 0);
}

#[tokio::test]
async fn inner_error_and_cancelled_handler_release_without_refunding() {
    let l = limiter();
    let inner = tower::service_fn(|_: Request<()>| async {
        Err::<Response<Full<bytes::Bytes>>, _>("backend")
    });
    let service = RateLimitLayer::new(policy(l.clone()), |e, _: &Parts| {
        rejection_response::<Full<_>>(e)
    })
    .layer(inner);
    assert!(service.oneshot(Request::new(())).await.is_err());
    assert_eq!(l.stats().active, 0);
    let inner = tower::service_fn(|_: Request<()>| {
        std::future::pending::<Result<Response<Full<bytes::Bytes>>, Infallible>>()
    });
    let service = RateLimitLayer::new(policy(l.clone()), |e, _: &Parts| {
        rejection_response::<Full<_>>(e)
    })
    .layer(inner);
    let mut future = Box::pin(service.oneshot(Request::new(())));
    assert!(futures_util::poll!(future.as_mut()).is_pending());
    assert_eq!(l.stats().active, 1);
    drop(future);
    assert_eq!(l.stats().active, 0);
    for _ in 0..8 {
        l.check((), NonZeroU32::new(1).unwrap()).unwrap();
    }
    assert!(l.check((), NonZeroU32::new(1).unwrap()).is_err());
}

struct ReadyService {
    ready: bool,
    calls: Arc<AtomicUsize>,
}
impl Clone for ReadyService {
    fn clone(&self) -> Self {
        Self {
            ready: false,
            calls: self.calls.clone(),
        }
    }
}
impl Service<Request<()>> for ReadyService {
    type Response = Response<Full<bytes::Bytes>>;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.ready = true;
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Request<()>) -> Self::Future {
        assert!(self.ready);
        self.ready = false;
        assert_eq!(req.uri(), "/kept");
        assert_eq!(req.headers()["x-original"], "yes");
        assert_eq!(req.extensions().get::<u32>(), Some(&17));
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(Response::new(Full::new(axum_bytes(b"ok")))))
    }
}
#[tokio::test]
async fn uses_ready_instance_preserves_parts_and_does_not_exempt_options() {
    let calls = Arc::new(AtomicUsize::new(0));
    let policy = AsyncPolicy::from_sync(Policy::new(|parts: &Parts| {
        if parts.method == http::Method::OPTIONS {
            Err("quota")
        } else {
            Ok(Admission::unrestricted())
        }
    }));
    let service = RateLimitLayer::new(policy, |_: &str, parts: &Parts| {
        assert_eq!(parts.method, http::Method::OPTIONS);
        Response::builder()
            .status(429)
            .body(Full::new(axum_bytes(b"denied")))
            .unwrap()
    })
    .layer(ReadyService {
        ready: false,
        calls: calls.clone(),
    });
    let mut request = Request::builder()
        .uri("/kept")
        .header("x-original", "yes")
        .body(())
        .unwrap();
    request.extensions_mut().insert(17u32);
    service.clone().oneshot(request).await.unwrap();
    let response = service
        .oneshot(Request::builder().method("OPTIONS").body(()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 429);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rejection_never_polls_request_body_or_invokes_handler_and_keeps_backend_errors_distinct() {
    struct Untouched;
    impl http_body::Body for Untouched {
        type Data = bytes::Bytes;
        type Error = Infallible;
        fn poll_frame(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
            panic!("must not read body")
        }
    }
    let inner = tower::service_fn(|_: Request<Untouched>| async {
        panic!("must not run handler");
        #[allow(unreachable_code)]
        Ok::<_, Infallible>(Response::new(Full::new(axum_bytes(b""))))
    });
    let policy = AsyncPolicy::new(|_: &Parts| {
        Box::pin(async { Err::<Admission, _>("storage unavailable") })
    });
    let service = RateLimitLayer::new(policy, |e, _: &Parts| {
        assert_eq!(e, "storage unavailable");
        Response::builder()
            .status(503)
            .body(Full::new(axum_bytes(b"unavailable")))
            .unwrap()
    })
    .layer(inner);
    assert_eq!(
        service
            .oneshot(Request::new(Untouched))
            .await
            .unwrap()
            .status(),
        503
    );
}

#[cfg(feature = "http")]
#[tokio::test]
async fn real_http_denial_retry_and_public_routes() {
    use simple_server::axum::{self, Router, body::Body, routing::post};
    let l = KeyedLimiter::new(
        Quota::replenishing(Duration::from_secs(60), NonZeroU32::new(1).unwrap()).unwrap(),
        StoreConfig::bounded(NonZeroUsize::new(4).unwrap(), Duration::ZERO),
    );
    let checks = AsyncPolicy::from_sync(Policy::new(move |parts: &Parts| {
        let key = parts
            .headers
            .get("x-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_owned();
        l.check(key, NonZeroU32::new(1).unwrap())?;
        Ok(Admission::unrestricted())
    }));
    let app = Router::new()
        .route("/limited", post(|body: String| async move { body }))
        .layer(RateLimitLayer::new(checks, |e, _: &Parts| {
            rejection_response::<Body>(e)
        }))
        .route("/public", post(|| async { "public" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    // Raw HTTP avoids adding a client dependency to the shared crate.
    async fn send(addr: std::net::SocketAddr, path: &str, key: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nx-key: {key}\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata").as_bytes()).await.unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).await.unwrap();
        response
    }
    let ok = send(addr, "/limited", "a").await;
    assert!(ok.starts_with("HTTP/1.1 200"));
    assert!(ok.ends_with("data"));
    let denied = send(addr, "/limited", "a").await;
    assert!(denied.starts_with("HTTP/1.1 429"));
    assert!(denied.contains("retry-after: 60"));
    assert!(
        send(addr, "/limited", "b")
            .await
            .starts_with("HTTP/1.1 200")
    );
    assert!(send(addr, "/public", "a").await.starts_with("HTTP/1.1 200"));
    task.abort();
    let _ = task.await;
}
