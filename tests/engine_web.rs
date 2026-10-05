#![cfg(all(feature = "engine-web", feature = "client"))]
use http_body_util::BodyExt;
use simple_server::{
    client::Client,
    engine_web::*,
    runtime::{spawn, spawn_blocking},
    time::{sleep, timeout},
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn client() -> Client {
    Client::builder().no_proxy().no_redirect().build().unwrap()
}
async fn start(
    router: Router,
) -> (
    String,
    Shutdown,
    simple_server::runtime::JoinHandle<io::Result<()>>,
) {
    let listener = bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr());
    let stop = Shutdown::new();
    (url, stop.clone(), spawn(serve(listener, router, stop)))
}
async fn finish(stop: Shutdown, server: simple_server::runtime::JoinHandle<io::Result<()>>) {
    stop.request();
    timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
#[simple_server::test]
async fn routing_metadata_echo_and_binary_duplicate_headers_survive_handler_return() {
    let methods = MethodRouter::new()
        .unwrap()
        .on(Method::POST, |request| async move {
            let metadata = request.extensions().get::<RequestMetadata>().unwrap();
            assert_eq!(metadata.matched_path.as_deref(), Some("/api/echo/{name}"));
            assert_eq!(
                metadata.path_params,
                [("name".into(), "hello world".into())]
            );
            assert_eq!(
                metadata.original_uri.as_ref().unwrap().path(),
                "/api/echo/hello%20world"
            );
            assert!(metadata.peer.unwrap().ip().is_loopback());
            assert!(metadata.path_error.is_none());
            let headers = request.headers().clone();
            let mut response = Response::new(request.into_body());
            *response.status_mut() = StatusCode::CREATED;
            for value in headers.get_all("x-duplicate") {
                response.headers_mut().append("x-duplicate", value.clone());
            }
            response
        })
        .unwrap();
    let inner = Router::new()
        .unwrap()
        .route("/echo/{name}", methods)
        .unwrap();
    let router = Router::new().unwrap().nest("/api", inner).unwrap();
    let (url, stop, server) = start(router).await;
    let mut headers = HeaderMap::new();
    headers.append("x-duplicate", HeaderValue::from_bytes(&[0x80]).unwrap());
    headers.append("x-duplicate", HeaderValue::from_static("second"));
    let response = client()
        .post(format!("{url}/api/echo/hello%20world?x=1"))
        .headers(headers)
        .body(vec![0, 1, 255])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response
            .headers()
            .get_all("x-duplicate")
            .iter()
            .map(|v| v.as_bytes().to_vec())
            .collect::<Vec<_>>(),
        [vec![0x80], b"second".to_vec()]
    );
    assert_eq!(response.bytes().await.unwrap().as_ref(), &[0, 1, 255]);
    finish(stop, server).await;
}
#[simple_server::test]
async fn methods_fallback_merge_and_clones_preserve_native_routing_semantics() {
    let base = Router::new()
        .unwrap()
        .route(
            "/get",
            MethodRouter::new()
                .unwrap()
                .on(Method::GET, |_| async { Response::new(Body::from("get")) })
                .unwrap(),
        )
        .unwrap();
    let extended = base
        .clone()
        .merge(
            Router::new()
                .unwrap()
                .route(
                    "/extra",
                    MethodRouter::any(|_| async { Response::new(Body::from("extra")) }).unwrap(),
                )
                .unwrap(),
        )
        .unwrap()
        .fallback(|_| async {
            let mut r = Response::new(Body::from("fallback"));
            *r.status_mut() = StatusCode::NOT_FOUND;
            r
        })
        .unwrap();
    let (url, stop, server) = start(extended).await;
    assert_eq!(
        client()
            .head(format!("{url}/get"))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap()
            .len(),
        0
    );
    let response = client().post(format!("{url}/get")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(
        response.headers()["allow"]
            .to_str()
            .unwrap()
            .contains("GET")
    );
    assert_eq!(
        client()
            .get(format!("{url}/missing"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "fallback"
    );
    assert_eq!(
        client()
            .put(format!("{url}/extra"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "extra"
    );
    finish(stop, server).await;
    let (url, stop, server) = start(base).await;
    assert_eq!(
        client()
            .get(format!("{url}/extra"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    finish(stop, server).await;
}
#[simple_server::test]
async fn collection_limits_stream_errors_and_trailers_remain_observable() {
    assert!(Body::from("too long").collect(2).await.is_err());
    assert_eq!(Body::from("ok").collect(2).await.unwrap(), "ok");
    let mut trailers = HeaderMap::new();
    trailers.insert("x-end", HeaderValue::from_static("yes"));
    let frames = futures_util::stream::iter([
        Ok::<_, io::Error>(Frame::data(Bytes::from_static(b"data"))),
        Ok(Frame::trailers(trailers)),
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
            .unwrap()["x-end"],
        "yes"
    );
    let body = Body::from_stream(futures_util::stream::iter([Err::<Bytes, _>(
        io::Error::other("broken"),
    )]));
    assert!(
        body.collect(10)
            .await
            .unwrap_err()
            .to_string()
            .contains("broken")
    );
}
#[simple_server::test]
async fn shutdown_drains_a_live_response_without_collecting_it() {
    use futures_util::StreamExt;
    let release = Shutdown::new();
    let gate = release.clone();
    let router = Router::new()
        .unwrap()
        .fallback(move |_| {
            let gate = gate.clone();
            async move {
                let first =
                    futures_util::stream::iter([Ok::<_, io::Error>(Bytes::from_static(b"first"))]);
                let last = futures_util::stream::once(async move {
                    gate.requested().await;
                    Ok::<_, io::Error>(Bytes::from_static(b"last"))
                });
                Response::new(Body::from_stream(first.chain(last)))
            }
        })
        .unwrap();
    let (url, stop, mut server) = start(router).await;
    let mut response = client().get(url).send().await.unwrap();
    assert_eq!(response.chunk().await.unwrap().unwrap(), "first");
    stop.request();
    assert!(
        timeout(Duration::from_millis(20), &mut server)
            .await
            .is_err()
    );
    release.request();
    assert_eq!(response.bytes().await.unwrap(), "last");
    finish(stop, server).await;
}
struct DropStream {
    first: bool,
    dropped: Arc<AtomicBool>,
}
impl Drop for DropStream {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
impl futures_util::Stream for DropStream {
    type Item = Result<Bytes, io::Error>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if self.first {
            self.first = false;
            std::task::Poll::Ready(Some(Ok(Bytes::from_static(b"first"))))
        } else {
            std::task::Poll::Pending
        }
    }
}
#[simple_server::test]
async fn disconnected_client_releases_pending_response_body() {
    let dropped = Arc::new(AtomicBool::new(false));
    let observed = dropped.clone();
    let router = Router::new()
        .unwrap()
        .fallback(move |_| {
            let dropped = dropped.clone();
            async move {
                Response::new(Body::from_stream(DropStream {
                    first: true,
                    dropped,
                }))
            }
        })
        .unwrap();
    let (url, stop, server) = start(router).await;
    let mut response = client().get(url).send().await.unwrap();
    response.chunk().await.unwrap().unwrap();
    drop(response);
    timeout(Duration::from_secs(3), async {
        while !observed.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    finish(stop, server).await;
}
#[simple_server::test]
async fn request_and_response_trailers_cross_the_public_body_boundary() {
    let router = Router::new()
        .unwrap()
        .fallback(|request| async move {
            let mut response = Response::new(request.into_body());
            response
                .headers_mut()
                .insert("trailer", HeaderValue::from_static("x-end"));
            response
        })
        .unwrap();
    let listener = bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr();
    let stop = Shutdown::new();
    let server = spawn(serve(listener, router, stop.clone()));
    let output=spawn_blocking(move || {
        use std::io::{Read,Write};
        let mut stream=std::net::TcpStream::connect(address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        stream.write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nTE: trailers\r\nTransfer-Encoding: chunked\r\nTrailer: x-end\r\n\r\n4\r\ndata\r\n0\r\nx-end: yes\r\n\r\n").unwrap();
        let mut output=String::new(); stream.read_to_string(&mut output).unwrap(); output
    }).await.unwrap();
    assert!(output.contains("data"), "{output}");
    assert!(output.contains("x-end: yes"), "{output}");
    finish(stop, server).await;
}
#[simple_server::test]
async fn listener_errors_drop_and_already_requested_shutdown_release_sockets() {
    let listener = bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().to_string();
    assert_eq!(
        bind(&address).await.err().unwrap().kind(),
        io::ErrorKind::AddrInUse
    );
    drop(listener);
    let listener = bind(&address).await.unwrap();
    let stop = Shutdown::new();
    stop.request();
    timeout(
        Duration::from_secs(1),
        serve(listener, Router::new().unwrap(), stop),
    )
    .await
    .unwrap()
    .unwrap();
    drop(bind(&address).await.unwrap());
}
#[simple_server::test]
async fn invalid_routes_and_handler_panics_do_not_break_other_requests() {
    let router = Router::new().unwrap();
    assert!(
        router
            .clone()
            .route("invalid", MethodRouter::new().unwrap())
            .is_err()
    );
    let router = router
        .route(
            "/panic",
            MethodRouter::any(|_| async { panic!("handler panic") }).unwrap(),
        )
        .unwrap()
        .fallback(|_| async { Response::new(Body::from("alive")) })
        .unwrap();
    let (url, stop, server) = start(router).await;
    assert_eq!(
        client()
            .get(format!("{url}/panic"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        client()
            .get(format!("{url}/ok"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "alive"
    );
    finish(stop, server).await;
}
