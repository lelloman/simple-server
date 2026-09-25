#![cfg(feature = "web")]
use simple_server::web::{
    self, Body, Extension, IntoResponse, MatchedPath, Request, Router, State, StatusCode,
    middleware::{self, Next},
    routing::get,
};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[tokio::test]
async fn layers_preserve_order_metadata_and_response_extensions() {
    type Log = Arc<Mutex<Vec<&'static str>>>;
    async fn outer(State(log): State<Log>, request: Request, next: Next) -> web::Response {
        log.lock().unwrap().push("outer in");
        assert_eq!(
            request.extensions().get::<MatchedPath>().unwrap().as_str(),
            "/api/item/{id}"
        );
        let response = next.run(request).await;
        log.lock().unwrap().push("outer out");
        response
    }
    async fn inner(Extension(log): Extension<Log>, request: Request, next: Next) -> web::Response {
        log.lock().unwrap().push("inner in");
        let mut response = next.run(request).await;
        log.lock().unwrap().push("inner out");
        response.extensions_mut().insert(42u32);
        response
    }
    let log = Log::default();
    let router = Router::new()
        .nest(
            "/api",
            Router::new().route("/item/{id}", get(|| async { "ok" })),
        )
        .layer(middleware::from_fn(inner))
        .layer(Extension(log.clone()))
        .layer(middleware::from_fn_with_state(log.clone(), outer));
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/item/secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.extensions().get::<u32>(), Some(&42));
    assert_eq!(response.into_body().collect(10).await.unwrap(), "ok");
    assert_eq!(
        *log.lock().unwrap(),
        ["outer in", "inner in", "inner out", "outer out"]
    );
}

#[tokio::test]
async fn route_layer_rejects_without_reading_body_but_leaves_404_alone() {
    async fn deny(_: Request, _: Next) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
    let router = Router::new()
        .route(
            "/private",
            get(|| async {
                panic!("handler must not run");
                #[allow(unreachable_code)]
                "bad"
            }),
        )
        .route_layer(middleware::from_fn(deny));
    for (path, status) in [
        ("/private", StatusCode::UNAUTHORIZED),
        ("/missing", StatusCode::NOT_FOUND),
    ] {
        let stream = futures_util::stream::poll_fn(
            |_| -> std::task::Poll<Option<Result<web::Bytes, std::io::Error>>> {
                panic!("body must not be read")
            },
        );
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::from_stream(stream))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
}

#[tokio::test]
async fn missing_extension_rejects_before_middleware_runs() {
    async fn middleware(Extension(_): Extension<u32>, _: Request, _: Next) -> StatusCode {
        panic!("missing extension");
    }
    let router = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(middleware::from_fn(middleware));
    let response = router.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn fallback_service_preserves_status_headers_and_stream() {
    let service = tower::service_fn(|_: Request| async {
        let body = Body::from_stream(futures_util::stream::iter([
            Ok::<_, std::io::Error>("one"),
            Ok("two"),
        ]));
        let mut response = (StatusCode::ACCEPTED, body).into_response();
        response
            .headers_mut()
            .insert("x-fallback", "yes".parse().unwrap());
        Ok::<_, std::convert::Infallible>(response)
    });
    let router = Router::new().fallback_service(service);
    let response = router.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.headers()["x-fallback"], "yes");
    assert_eq!(response.into_body().collect(10).await.unwrap(), "onetwo");
}

#[cfg(feature = "lifecycle")]
#[tokio::test]
async fn real_server_inserts_direct_peer_and_shuts_down() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn peer(web::ConnectInfo(peer): web::ConnectInfo<std::net::SocketAddr>) -> String {
        peer.to_string()
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = simple_server::lifecycle::Shutdown::new();
    let server = tokio::spawn(web::serve_with_connect_info(
        listener,
        Router::new().route("/", get(peer)),
        shutdown.clone(),
    ));
    let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
    let peer = client.local_addr().unwrap();
    client.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nX-Forwarded-For: 203.0.113.1\r\n\r\n").await.unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).await.unwrap();
    assert!(response.ends_with(&peer.to_string()), "{response}");
    shutdown.request();
    tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[derive(Clone)]
struct ReadyService {
    ready: bool,
}
impl tower::Service<Request> for ReadyService {
    type Response = web::Response;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<web::Response, Self::Error>>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.ready = true;
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, _: Request) -> Self::Future {
        assert!(self.ready, "called before readiness");
        self.ready = false;
        std::future::ready(Ok("ready".into_response()))
    }
}
#[tokio::test]
async fn middleware_next_and_fallback_respect_readiness() {
    use tower::Layer;
    async fn pass(request: Request, next: Next) -> web::Response {
        next.run(request).await
    }
    let service = middleware::from_fn(pass).layer(ReadyService { ready: false });
    let response = service.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(response.into_body().collect(10).await.unwrap(), "ready");
    let response = Router::new()
        .fallback_service(ReadyService { ready: false })
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn streaming_body_is_lazy_and_dropped_when_response_is_cancelled() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Stream(Arc<AtomicBool>);
    impl futures_util::Stream for Stream {
        type Item = Result<web::Bytes, std::io::Error>;
        fn poll_next(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            panic!("stream should not be polled before the client reads it")
        }
    }
    impl Drop for Stream {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let state = dropped.clone();
    let router = Router::new().route(
        "/",
        get(move || async move { Body::from_stream(Stream(state)) }),
    );
    let response = router.oneshot(Request::new(Body::empty())).await.unwrap();
    assert!(!dropped.load(Ordering::SeqCst));
    drop(response);
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn router_layers_accept_standard_response_bodies() {
    let layer = tower::util::MapResponseLayer::new(|response: web::Response| {
        response.map(|_| http_body_util::Full::new(web::Bytes::from_static(b"replaced")))
    });
    let router = Router::new()
        .route("/", get(|| async { StatusCode::ACCEPTED }))
        .layer(layer);
    let response = router.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.into_body().collect(20).await.unwrap(), "replaced");
}

#[tokio::test]
async fn standard_body_adapter_preserves_trailers_and_errors() {
    use http_body_util::BodyExt;
    let mut trailers = web::HeaderMap::new();
    trailers.insert("x-checksum", "value".parse().unwrap());
    let frames = futures_util::stream::iter([
        Ok::<_, std::io::Error>(http_body::Frame::data(web::Bytes::from_static(b"data"))),
        Ok(http_body::Frame::trailers(trailers)),
        Err(std::io::Error::other("stream failure")),
    ]);
    let mut body = Body::new(http_body_util::StreamBody::new(frames));
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "data"
    );
    assert_eq!(
        body.frame()
            .await
            .unwrap()
            .unwrap()
            .into_trailers()
            .unwrap()["x-checksum"],
        "value"
    );
    assert!(
        body.frame()
            .await
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("stream failure")
    );
}
