#![cfg(all(feature = "engine-web", feature = "client"))]
use simple_server::{
    client::Client,
    engine_web::{self as e, middleware as m},
    runtime::spawn,
    time::{sleep, timeout},
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone)]
struct Marker(Arc<AtomicUsize>);
impl Drop for Marker {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[derive(Clone, Debug)]
struct Label(String);
async fn start(
    router: e::Router,
) -> (
    String,
    e::Shutdown,
    simple_server::runtime::JoinHandle<io::Result<()>>,
) {
    let listener = e::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr());
    let stop = e::Shutdown::new();
    (url, stop.clone(), spawn(e::serve(listener, router, stop)))
}
async fn finish(stop: e::Shutdown, server: simple_server::runtime::JoinHandle<io::Result<()>>) {
    stop.request();
    timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
fn client() -> Client {
    Client::builder().no_proxy().no_redirect().build().unwrap()
}
async fn inner(mut request: e::Request, next: m::Next) -> e::Response {
    assert_eq!(request.extensions().get::<Label>().unwrap().0, "outer");
    request.extensions_mut().insert(Label("inner".into()));
    let mut response = next.run(request).await;
    assert_eq!(response.extensions().get::<Label>().unwrap().0, "handler");
    response
        .extensions_mut()
        .insert(Label("response-inner".into()));
    response
        .headers_mut()
        .append("x-order", "inner".parse().unwrap());
    response
}
async fn outer(mut request: e::Request, next: m::Next) -> e::Response {
    request.extensions_mut().insert(Label("outer".into()));
    let mut response = next.run(request).await;
    assert_eq!(
        response.extensions().get::<Label>().unwrap().0,
        "response-inner"
    );
    response
        .headers_mut()
        .append("x-order", "outer".parse().unwrap());
    response
}
#[simple_server::test]
async fn layered_extensions_and_pending_state_survive_native_continuations() {
    let router = e::Router::<String>::new()
        .unwrap()
        .route(
            "/items/{id}",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(
                    e::Method::POST,
                    |e::State(state): e::State<String>,
                     e::Extension(label): e::Extension<Label>,
                     e::Path(id): e::Path<u32>,
                     request: e::Request| async move {
                        assert_eq!(label.0, "inner");
                        let mut response = e::Response::new(request.into_body());
                        response
                            .headers_mut()
                            .insert("x-state", state.parse().unwrap());
                        response
                            .headers_mut()
                            .insert("x-id", id.to_string().parse().unwrap());
                        response.extensions_mut().insert(Label("handler".into()));
                        response
                    },
                )
                .unwrap(),
        )
        .unwrap()
        .layer(m::from_fn(inner))
        .unwrap()
        .layer(m::from_fn(outer))
        .unwrap();
    let (url, stop, server) = start(router.clone().with_state("one".to_owned()).unwrap()).await;
    let (url2, stop2, server2) = start(router.with_state("two".to_owned()).unwrap()).await;
    for (base, state) in [(url, "one"), (url2, "two")] {
        let response = client()
            .post(format!("{base}/items/42"))
            .body("stream")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["x-state"], state);
        assert_eq!(response.headers()["x-id"], "42");
        assert_eq!(
            response
                .headers()
                .get_all("x-order")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["inner", "outer"]
        );
        assert_eq!(response.text().await.unwrap(), "stream");
    }
    finish(stop, server).await;
    finish(stop2, server2).await;
}

#[simple_server::test]
async fn layers_include_native_errors_but_not_routes_added_later() {
    let router = e::Router::new()
        .unwrap()
        .route(
            "/first",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, || async { "first" })
                .unwrap(),
        )
        .unwrap()
        .layer(m::map_response(|mut response: e::Response| async move {
            response
                .headers_mut()
                .insert("x-layer", "yes".parse().unwrap());
            response
        }))
        .unwrap()
        .route(
            "/later",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, || async { "later" })
                .unwrap(),
        )
        .unwrap();
    let (url, stop, server) = start(router).await;
    for (path, method, status, layered) in [
        ("/first", e::Method::GET, 200, true),
        ("/first", e::Method::HEAD, 200, true),
        ("/first", e::Method::POST, 405, true),
        ("/missing", e::Method::GET, 404, true),
        ("/later", e::Method::GET, 200, false),
    ] {
        let response = client()
            .request(method, format!("{url}{path}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        assert_eq!(response.headers().contains_key("x-layer"), layered);
        if status == 405 {
            assert!(
                response.headers()["allow"]
                    .to_str()
                    .unwrap()
                    .contains("GET")
            );
        }
    }
    finish(stop, server).await;
}

#[simple_server::test]
async fn nested_layers_keep_original_uri_and_captures_and_can_short_circuit() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let inner = e::Router::new()
        .unwrap()
        .route(
            "/items/{id}",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, move |request| {
                    let observed = observed.clone();
                    async move {
                        observed.fetch_add(1, Ordering::SeqCst);
                        let metadata = request.extensions().get::<e::RequestMetadata>().unwrap();
                        assert_eq!(
                            metadata.original_uri.as_ref().unwrap().path(),
                            "/api/items/42"
                        );
                        assert_eq!(metadata.path_params, [("id".into(), "42".into())]);
                        assert_eq!(request.extensions().get::<Label>().unwrap().0, "nested");
                        e::Response::new(e::Body::from(request.uri().path().to_owned()))
                    }
                })
                .unwrap(),
        )
        .unwrap()
        .layer(m::from_fn(
            |mut request: e::Request, next: m::Next| async move {
                if request.headers().contains_key("x-deny") {
                    let mut response = e::Response::new(e::Body::empty());
                    *response.status_mut() = e::StatusCode::FORBIDDEN;
                    return response;
                }
                request.extensions_mut().insert(Label("nested".into()));
                next.run(request).await
            },
        ))
        .unwrap();
    let router = e::Router::new().unwrap().nest("/api", inner).unwrap();
    let (url, stop, server) = start(router).await;
    assert_eq!(
        client()
            .get(format!("{url}/api/items/42"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "/items/42"
    );
    assert_eq!(
        client()
            .get(format!("{url}/api/items/42"))
            .header("x-deny", "yes")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    finish(stop, server).await;
}

#[simple_server::test]
async fn dropping_a_client_releases_waiting_host_extensions() {
    let drops = Arc::new(AtomicUsize::new(0));
    let observed = drops.clone();
    let entered = Arc::new(AtomicUsize::new(0));
    let entering = entered.clone();
    let router = e::Router::new()
        .unwrap()
        .route(
            "/",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, move |request| {
                    let entering = entering.clone();
                    async move {
                        entering.fetch_add(1, Ordering::SeqCst);
                        std::future::pending::<()>().await;
                        e::Response::new(request.into_body())
                    }
                })
                .unwrap(),
        )
        .unwrap()
        .layer(m::from_fn(move |mut request: e::Request, next: m::Next| {
            let observed = observed.clone();
            async move {
                request.extensions_mut().insert(Marker(observed));
                next.run(request).await
            }
        }))
        .unwrap();
    let (url, stop, server) = start(router).await;
    let task = spawn(async move { client().get(url).send().await });
    timeout(Duration::from_secs(3), async {
        while entered.load(Ordering::SeqCst) == 0 {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    task.abort();
    let _ = task.await;
    timeout(Duration::from_secs(3), async {
        while drops.load(Ordering::SeqCst) == 0 {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    finish(stop, server).await;
}

#[cfg(feature = "web")]
#[simple_server::test]
async fn router_layer_status_headers_body_and_scope_match_source() {
    use simple_server::web as w;
    async fn source_handler(request: w::Request) -> w::Response {
        let label = request.extensions().get::<Label>().unwrap().0.clone();
        let mut response = w::Response::new(w::Body::from(format!("{label}:{}", request.uri())));
        response
            .headers_mut()
            .insert("x-request", request.headers()["x-request"].clone());
        response.extensions_mut().insert(Label("reply".into()));
        response
    }
    async fn engine_handler(request: e::Request) -> e::Response {
        let label = request.extensions().get::<Label>().unwrap().0.clone();
        let mut response = e::Response::new(e::Body::from(format!("{label}:{}", request.uri())));
        response
            .headers_mut()
            .insert("x-request", request.headers()["x-request"].clone());
        response.extensions_mut().insert(Label("reply".into()));
        response
    }
    macro_rules! layer {
        ($web:path, $middleware:path) => {{
            use $middleware as mid;
            use $web as web;
            mid::from_fn(|mut request: web::Request, next: mid::Next| async move {
                request.extensions_mut().insert(Label("request".into()));
                request
                    .headers_mut()
                    .insert("x-request", "modified".parse().unwrap());
                if request.uri().query() == Some("rewrite") {
                    *request.uri_mut() = "/rewritten".parse().unwrap();
                }
                let mut response = next.run(request).await;
                let label = response
                    .extensions()
                    .get::<Label>()
                    .map(|v| v.0.as_str())
                    .unwrap_or("native")
                    .to_owned();
                response
                    .headers_mut()
                    .append("x-layer", label.parse().unwrap());
                response
                    .headers_mut()
                    .append("x-layer", "second".parse().unwrap());
                response
            })
        }};
    }
    let source = w::Router::new()
        .route("/echo", w::routing::get(source_handler))
        .layer(layer!(simple_server::web, simple_server::web::middleware))
        .route("/later", w::routing::get(|| async { "later" }));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let source_url = format!("http://{}", listener.local_addr().unwrap());
    let source_stop = simple_server::lifecycle::Shutdown::new();
    let signal = source_stop.clone();
    let source_server = simple_server::runtime::spawn_blocking(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                w::serve(
                    simple_server::net::TcpListener::from_std(listener).unwrap(),
                    source,
                    signal,
                )
                .await
            })
    });
    let engine = e::Router::new()
        .unwrap()
        .route(
            "/echo",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, engine_handler)
                .unwrap(),
        )
        .unwrap()
        .layer(layer!(
            simple_server::engine_web,
            simple_server::engine_web::middleware
        ))
        .unwrap()
        .route(
            "/later",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, || async { "later" })
                .unwrap(),
        )
        .unwrap();
    let (url, stop, server) = start(engine).await;
    for (method, path) in [
        ("GET", "/echo"),
        ("HEAD", "/echo"),
        ("POST", "/echo"),
        ("GET", "/missing"),
        ("CUSTOM", "/missing"),
        ("GET", "/echo?rewrite"),
        ("GET", "/later"),
    ] {
        let mut observations = Vec::new();
        for base in [&source_url, &url] {
            let response = client()
                .request(method.parse().unwrap(), format!("{base}{path}"))
                .send()
                .await
                .unwrap();
            let status = response.status();
            let mut headers = response.headers().clone();
            headers.remove("date");
            observations.push((status, headers, response.bytes().await.unwrap()));
        }
        assert_eq!(observations[0], observations[1], "{method} {path}");
    }
    finish(stop, server).await;
    source_stop.request();
    timeout(Duration::from_secs(3), source_server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[cfg(unix)]
#[simple_server::test]
async fn continuation_streams_before_upload_finishes_and_retains_trailers() {
    use futures_util::StreamExt;
    use http_body_util::BodyExt;
    let router = e::Router::new()
        .unwrap()
        .fallback(|request| async {
            let mut response = e::Response::new(request.into_body());
            response
                .headers_mut()
                .insert("trailer", "x-end".parse().unwrap());
            response
        })
        .unwrap()
        .layer(m::from_fn(
            |request: e::Request, next: m::Next| async move { next.run(request).await },
        ))
        .unwrap()
        .layer(m::map_response(
            |response: e::Response| async move { response },
        ))
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("http.sock");
    let listener = e::unix::bind(&path).await.unwrap();
    let stop = e::Shutdown::new();
    let server = spawn(e::unix::serve(listener, router, stop.clone()));
    let release = e::Shutdown::new();
    let gate = release.clone();
    let mut trailers = e::HeaderMap::new();
    trailers.insert("x-end", "done".parse().unwrap());
    let frames = futures_util::stream::iter([Ok::<_, io::Error>(e::Frame::data(
        e::Bytes::from_static(b"first"),
    ))])
    .chain(futures_util::stream::once(async move {
        gate.requested().await;
        Ok(e::Frame::trailers(trailers))
    }));
    let request = http::Request::builder()
        .method("POST")
        .uri("/")
        .header("te", "trailers")
        .header("trailer", "x-end")
        .body(e::Body::new(http_body_util::StreamBody::new(frames)))
        .unwrap();
    let mut response = timeout(
        Duration::from_secs(3),
        e::unix::UnixClient::new().unwrap().request(&path, request),
    )
    .await
    .unwrap()
    .unwrap();
    let first = timeout(Duration::from_secs(3), response.body_mut().frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    assert_eq!(first, "first");
    release.request();
    let rest = timeout(
        Duration::from_secs(3),
        BodyExt::collect(response.into_body()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(rest.trailers().unwrap()["x-end"], "done");
    finish(stop, server).await;
}

#[simple_server::test]
async fn concurrent_contexts_stay_isolated_and_release_route_captures() {
    let captured = Arc::new(());
    let handler_capture = captured.clone();
    let router = e::Router::new()
        .unwrap()
        .route(
            "/{id}",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, move |request| {
                    let keep = handler_capture.clone();
                    async move {
                        simple_server::runtime::yield_now().await;
                        let id = request.extensions().get::<Label>().unwrap().0.clone();
                        let mut response = e::Response::new(e::Body::from(id.clone()));
                        response.extensions_mut().insert(Label(id));
                        drop(keep);
                        response
                    }
                })
                .unwrap(),
        )
        .unwrap()
        .layer(m::from_fn(
            |e::Path(id): e::Path<u32>, mut request: e::Request, next: m::Next| async move {
                request.extensions_mut().insert(Label(id.to_string()));
                let mut response = next.run(request).await;
                assert_eq!(
                    response.extensions().get::<Label>().unwrap().0,
                    id.to_string()
                );
                response
                    .headers_mut()
                    .insert("x-id", id.to_string().parse().unwrap());
                response
            },
        ))
        .unwrap();
    let (url, stop, server) = start(router).await;
    let mut tasks = Vec::new();
    for id in 0..16 {
        let url = url.clone();
        tasks.push(spawn(async move {
            let response = client().get(format!("{url}/{id}")).send().await.unwrap();
            assert_eq!(response.headers()["x-id"], id.to_string());
            assert_eq!(response.text().await.unwrap(), id.to_string());
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    finish(stop, server).await;
    timeout(Duration::from_secs(3), async {
        while Arc::strong_count(&captured) != 1 {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[simple_server::test]
async fn middleware_panics_are_isolated_and_short_circuit_releases_next() {
    let captured = Arc::new(());
    let keep = captured.clone();
    let router = e::Router::new()
        .unwrap()
        .fallback(move |_| {
            let keep = keep.clone();
            async move {
                drop(keep);
                e::Response::new(e::Body::from("ok"))
            }
        })
        .unwrap()
        .layer(m::from_fn(
            |request: e::Request, next: m::Next| async move {
                if request.uri().path() == "/panic" {
                    panic!("middleware panic probe");
                }
                if request.uri().path() == "/short" {
                    return e::Response::new(e::Body::from("short"));
                }
                next.run(request).await
            },
        ))
        .unwrap();
    let (url, stop, server) = start(router).await;
    assert_eq!(
        client()
            .get(format!("{url}/panic"))
            .send()
            .await
            .unwrap()
            .status(),
        500
    );
    assert_eq!(
        client()
            .get(format!("{url}/short"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "short"
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
        "ok"
    );
    finish(stop, server).await;
    timeout(Duration::from_secs(3), async {
        while Arc::strong_count(&captured) != 1 {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
