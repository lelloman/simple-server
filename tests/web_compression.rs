#![cfg(feature = "compression")]
use simple_server::web::{
    Body, HeaderMap, Request, Response, Router, StatusCode,
    compression::{CompressionLayer, CompressionQuality},
    routing::get,
};
use std::{
    convert::Infallible,
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::{Layer, ServiceExt, service_fn};

fn request(accept: Option<&str>) -> Request {
    let mut r = Request::builder();
    if let Some(v) = accept {
        r = r.header("accept-encoding", v);
    }
    r.body(Body::empty()).unwrap()
}
fn response(mime: &str, length: usize, extra: &[(&str, &str)], status: u16) -> Response {
    let mut r = Response::builder()
        .status(status)
        .header("content-type", mime);
    for (n, v) in extra {
        r = r.header(*n, *v);
    }
    r.body(Body::from(vec![b'a'; length])).unwrap()
}
fn decoded(headers: &HeaderMap, bytes: &[u8]) -> Vec<u8> {
    if headers.get("content-encoding").is_some_and(|v| v == "gzip") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .read_to_end(&mut out)
            .unwrap();
        out
    } else {
        bytes.to_vec()
    }
}

#[tokio::test]
async fn default_policy_matches_pezzottflix_legacy_gzip_layer() {
    for mime in [
        "application/json",
        "text/html",
        "image/png",
        "image/svg+xml",
        "application/grpc",
        "application/grpc-web",
        "text/event-stream",
        "application/octet-stream",
    ] {
        for length in [0, 31, 32, 4096] {
            for accept in [
                None,
                Some("gzip"),
                Some("identity"),
                Some("br, deflate"),
                Some("gzip;q=0"),
                Some("gzip;q=0.5, identity;q=1"),
                Some("*"),
                Some("GZIP"),
                Some("gzip, gzip;q=0"),
                Some("nonsense"),
            ] {
                for extra in [
                    vec![],
                    vec![("content-encoding", "gzip")],
                    vec![("content-range", "bytes 0-31/4096")],
                    vec![("vary", "Origin"), ("vary", "Accept-Encoding")],
                ] {
                    let service = || {
                        let extra = extra.clone();
                        service_fn(move |_: Request| {
                            let extra = extra.clone();
                            async move { Ok::<_, Infallible>(response(mime, length, &extra, 200)) }
                        })
                    };
                    let legacy =
                        tower_http_05::compression::CompressionLayer::new().layer(service());
                    let shared = CompressionLayer::new().layer(service());
                    let a = legacy.oneshot(request(accept)).await.unwrap();
                    let b: Response = shared.oneshot(request(accept)).await.unwrap();
                    assert_eq!(a.status(), b.status());
                    let vary = |headers: &HeaderMap| {
                        headers
                            .get_all("vary")
                            .iter()
                            .flat_map(|v| v.to_str().unwrap().split(','))
                            .map(|v| v.trim().to_ascii_lowercase())
                            .collect::<std::collections::BTreeSet<_>>()
                    };
                    assert_eq!(
                        vary(a.headers()),
                        vary(b.headers()),
                        "{mime} {length} {accept:?} {extra:?}"
                    );
                    let mut ah = a.headers().clone();
                    let mut bh = b.headers().clone();
                    ah.remove("vary");
                    bh.remove("vary");
                    assert_eq!(ah, bh, "{mime} {length} {accept:?} {extra:?}");
                    use http_body_util::BodyExt;
                    let a = a.into_body().collect().await.unwrap().to_bytes();
                    let b = b.into_body().collect(16384).await.unwrap();
                    assert_eq!(a, b, "{mime} {length} {accept:?} {extra:?}");
                }
            }
        }
    }
}

#[tokio::test]
async fn gzip_roundtrip_quality_disable_and_minimum_size() {
    for quality in [
        CompressionQuality::Default,
        CompressionQuality::Fastest,
        CompressionQuality::Best,
    ] {
        let layer = CompressionLayer::new().quality(quality);
        let service = service_fn(|_: Request| async {
            Ok::<_, Infallible>(response("application/json", 4096, &[], 200))
        });
        let r = layer
            .layer(service)
            .oneshot(request(Some("gzip")))
            .await
            .unwrap();
        assert_eq!(r.headers()["content-encoding"], "gzip");
        assert!(r.headers().get("content-length").is_none());
        let headers = r.headers().clone();
        let bytes = r.into_body().collect(16384).await.unwrap();
        assert_eq!(decoded(&headers, &bytes), vec![b'a'; 4096]);
    }
    for (layer, compressed) in [
        (CompressionLayer::new().gzip(false), false),
        (CompressionLayer::new().min_size(4097), false),
        (CompressionLayer::new().min_size(u64::MAX), false),
        (CompressionLayer::new().min_size(0), true),
    ] {
        let service = service_fn(|_: Request| async {
            Ok::<_, Infallible>(response("text/html", 4096, &[], 200))
        });
        let r = layer
            .layer(service)
            .oneshot(request(Some("gzip")))
            .await
            .unwrap();
        assert_eq!(r.headers().contains_key("content-encoding"), compressed);
        assert!(!r.into_body().collect(16384).await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn custom_predicate_receives_owned_metadata_and_never_reencodes_ranges() {
    let layer = CompressionLayer::new().compress_when(|status, _, headers, extensions| {
        status == StatusCode::CREATED
            && headers.contains_key("x-compress")
            && extensions.get::<u32>() == Some(&42)
    });
    for (status, extra, encoded) in [
        (201, vec![], true),
        (200, vec![], false),
        (201, vec![("content-range", "bytes 0-31/4096")], false),
        (201, vec![("content-encoding", "br")], true),
    ] {
        let service = service_fn(move |_: Request| {
            let extra = extra.clone();
            async move {
                let mut r = response("image/png", 4, &extra, status);
                r.headers_mut().insert("x-compress", "yes".parse().unwrap());
                r.extensions_mut().insert(42u32);
                Ok::<_, Infallible>(r)
            }
        });
        let r = layer
            .layer(service)
            .oneshot(request(Some("gzip")))
            .await
            .unwrap();
        assert_eq!(r.headers().contains_key("content-encoding"), encoded);
        if r.headers()
            .get("content-encoding")
            .is_some_and(|v| v == "br")
        {
            assert_eq!(
                r.into_body().collect(16384).await.unwrap().as_ref(),
                b"aaaa"
            );
        }
    }
    // Builder order is explicit: min_size restores MIME exclusions.
    let layer = CompressionLayer::new()
        .compress_when(|_, _, _, _| true)
        .min_size(0);
    let r = layer
        .layer(service_fn(|_: Request| async {
            Ok::<_, Infallible>(response("image/png", 4096, &[], 200))
        }))
        .oneshot(request(Some("gzip")))
        .await
        .unwrap();
    assert!(!r.headers().contains_key("content-encoding"));
}

struct DropProbe(Arc<AtomicUsize>);
impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[tokio::test]
async fn stream_is_lazy_and_dropping_response_releases_inner_body() {
    let polls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let service = service_fn({
        let polls = polls.clone();
        let drops = drops.clone();
        move |_: Request| {
            let polls = polls.clone();
            let guard = DropProbe(drops.clone());
            async move {
                let stream = futures_util::stream::poll_fn(move |_| {
                    let _keep_alive = &guard;
                    polls.fetch_add(1, Ordering::SeqCst);
                    std::task::Poll::Ready(Some(Ok::<_, std::io::Error>(vec![b'x'; 64])))
                });
                Ok::<_, Infallible>(
                    Response::builder()
                        .header("content-type", "text/plain")
                        .body(Body::from_stream(stream))
                        .unwrap(),
                )
            }
        }
    });
    let r = CompressionLayer::new()
        .layer(service)
        .oneshot(request(Some("gzip")))
        .await
        .unwrap();
    assert_eq!(r.headers()["content-encoding"], "gzip");
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    drop(r);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn streamed_chunks_roundtrip_and_body_errors_propagate() {
    for fails in [false, true] {
        let service = service_fn(move |_: Request| async move {
            let chunks = vec![
                Ok(vec![b'a'; 128]),
                if fails {
                    Err(std::io::Error::other("stream failure"))
                } else {
                    Ok(vec![b'b'; 128])
                },
            ];
            Ok::<_, Infallible>(
                Response::builder()
                    .header("content-type", "application/json")
                    .body(Body::from_stream(futures_util::stream::iter(chunks)))
                    .unwrap(),
            )
        });
        let r = CompressionLayer::new()
            .layer(service)
            .oneshot(request(Some("gzip")))
            .await
            .unwrap();
        let headers = r.headers().clone();
        let result = r.into_body().collect(16384).await;
        if fails {
            assert!(result.is_err());
        } else {
            assert_eq!(
                decoded(&headers, &result.unwrap()),
                [vec![b'a'; 128], vec![b'b'; 128]].concat()
            );
        }
    }
}

#[tokio::test]
async fn inner_service_errors_are_retained() {
    let service = service_fn(|_: Request| async { Err::<Response, _>("application failure") });
    let result = CompressionLayer::new()
        .layer(service)
        .oneshot(request(Some("gzip")))
        .await;
    assert_eq!(result.unwrap_err(), "application failure");
}

#[tokio::test]
async fn shared_router_accepts_the_owned_layer_and_head_contract() {
    let app = Router::new()
        .route("/", get(|| async { "x".repeat(4096) }))
        .layer(CompressionLayer::new());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("accept-encoding", "gzip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let headers = response.headers().clone();
    assert_eq!(
        decoded(
            &headers,
            &response.into_body().collect(16384).await.unwrap()
        ),
        vec![b'x'; 4096]
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/")
                .header("accept-encoding", "gzip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response
            .into_body()
            .collect(16384)
            .await
            .unwrap()
            .is_empty()
    );
}

#[cfg(feature = "test-harness")]
#[tokio::test]
async fn loopback_http_negotiation_roundtrip_and_head() {
    use simple_server::testing::TestServer;
    let app = Router::new()
        .route("/", get(|| async { "payload".repeat(1024) }))
        .layer(CompressionLayer::new());
    let server = TestServer::tcp(app).await.unwrap();
    let r = server
        .get("/")
        .header("accept-encoding", "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["content-encoding"], "gzip");
    assert_eq!(
        decoded(r.headers(), r.as_bytes()),
        "payload".repeat(1024).as_bytes()
    );
    let r = server
        .get("/")
        .header("accept-encoding", "gzip;q=0")
        .send()
        .await
        .unwrap();
    assert!(!r.headers().contains_key("content-encoding"));
    r.assert_text(&"payload".repeat(1024));
    assert!(
        server
            .head("/")
            .header("accept-encoding", "gzip")
            .send()
            .await
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    server.shutdown().await.unwrap();
}

#[test]
fn readiness_is_forwarded_without_calling_the_service() {
    use std::task::{Context, Poll};
    use tower::Service;
    #[derive(Clone)]
    struct Probe {
        ready: Arc<AtomicUsize>,
        calls: Arc<AtomicUsize>,
    }
    impl Service<Request> for Probe {
        type Response = Response;
        type Error = Infallible;
        type Future = std::future::Ready<Result<Response, Infallible>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
            if self.ready.load(Ordering::SeqCst) == 0 {
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
        fn call(&mut self, _: Request) -> Self::Future {
            self.calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(response("text/plain", 64, &[], 200)))
        }
    }
    let ready = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut service = CompressionLayer::new().layer(Probe {
        ready: ready.clone(),
        calls: calls.clone(),
    });
    let waker = futures_util::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(service.poll_ready(&mut cx).is_pending());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    ready.store(1, Ordering::SeqCst);
    assert!(matches!(service.poll_ready(&mut cx), Poll::Ready(Ok(()))));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn dropping_pending_response_future_cancels_inner_work() {
    use tower::Service;
    let drops = Arc::new(AtomicUsize::new(0));
    let service = service_fn({
        let drops = drops.clone();
        move |_: Request| {
            let guard = DropProbe(drops.clone());
            async move {
                let _guard = guard;
                std::future::pending::<Result<Response, Infallible>>().await
            }
        }
    });
    let mut service = CompressionLayer::new().layer(service);
    let future = service.call(request(Some("gzip")));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(future);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
