#![cfg(all(feature = "engine-web", feature = "client"))]
use simple_server::{
    client::Client,
    engine_web::{self as e, middleware as m},
    runtime::spawn,
    time::timeout,
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
async fn tag(mut response: e::Response) -> e::Response {
    response
        .headers_mut()
        .append("x-scope", "yes".parse().unwrap());
    response
}
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
fn router(scope: u8) -> e::Router {
    let methods = e::MethodRouter::new()
        .unwrap()
        .on_handler(e::Method::GET, || async { "get" })
        .unwrap();
    let methods = match scope {
        1 => methods.layer(m::map_response(tag)).unwrap(),
        2 => methods.route_layer(m::map_response(tag)).unwrap(),
        _ => methods,
    }
    .on_handler(e::Method::POST, || async { "post" })
    .unwrap();
    let router = e::Router::new()
        .unwrap()
        .route("/existing", methods)
        .unwrap();
    let router = if scope == 0 {
        router.route_layer(m::map_response(tag)).unwrap()
    } else {
        router
    };
    router
        .route(
            "/later",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, || async { "later" })
                .unwrap(),
        )
        .unwrap()
}
#[simple_server::test]
async fn scope_matrix_preserves_404_405_head_and_later_registrations() {
    for scope in 0..3 {
        let (url, stop, server) = start(router(scope)).await;
        for (method, path, status, tagged) in [
            ("GET", "/existing", 200, true),
            ("HEAD", "/existing", 200, true),
            ("POST", "/existing", 200, scope == 0),
            ("DELETE", "/existing", 405, scope != 2),
            ("GET", "/missing", 404, false),
            ("GET", "/later", 200, false),
        ] {
            let response = client()
                .request(method.parse().unwrap(), format!("{url}{path}"))
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status().as_u16(),
                status,
                "scope {scope}: {method} {path}"
            );
            assert_eq!(
                response.headers().contains_key("x-scope"),
                tagged,
                "scope {scope}: {method} {path}"
            );
            if status == 405 {
                let allow = response.headers()["allow"].to_str().unwrap();
                assert!(allow.contains("GET") && allow.contains("POST"));
            }
            if method == "HEAD" {
                assert!(response.bytes().await.unwrap().is_empty());
            }
        }
        finish(stop, server).await;
    }
}
#[simple_server::test]
async fn method_route_auth_does_not_turn_missing_methods_or_paths_into_denials() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let methods = e::MethodRouter::new()
        .unwrap()
        .on_handler(e::Method::GET, || async { "protected" })
        .unwrap()
        .route_layer(m::from_fn(move |_: e::Request, _: m::Next| {
            let observed = observed.clone();
            async move {
                observed.fetch_add(1, Ordering::SeqCst);
                e::StatusCode::UNAUTHORIZED
            }
        }))
        .unwrap();
    let (url, stop, server) = start(
        e::Router::new()
            .unwrap()
            .route("/protected", methods)
            .unwrap(),
    )
    .await;
    for (method, path, status) in [
        ("GET", "/protected", 401),
        ("HEAD", "/protected", 401),
        ("POST", "/protected", 405),
        ("GET", "/missing", 404),
    ] {
        assert_eq!(
            client()
                .request(method.parse().unwrap(), format!("{url}{path}"))
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            status
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    finish(stop, server).await;
}
#[simple_server::test]
async fn empty_route_layers_fail_eagerly_but_method_layer_can_wrap_fallback() {
    assert!(
        e::Router::<String>::new()
            .unwrap()
            .route_layer(m::map_response(tag))
            .is_err()
    );
    assert!(
        e::Router::<()>::new()
            .unwrap()
            .fallback(|_| async { e::Response::new(e::Body::empty()) })
            .unwrap()
            .route_layer(m::map_response(tag))
            .is_err()
    );
    assert!(
        e::MethodRouter::<String>::new()
            .unwrap()
            .route_layer(m::map_response(tag))
            .is_err()
    );
    assert!(
        e::MethodRouter::<()>::any(|_| async { e::Response::new(e::Body::empty()) })
            .unwrap()
            .route_layer(m::map_response(tag))
            .is_err()
    );
    let methods = e::MethodRouter::new()
        .unwrap()
        .layer(m::map_response(tag))
        .unwrap()
        .on_handler(e::Method::GET, || async { "late" })
        .unwrap();
    let (url, stop, server) = start(e::Router::new().unwrap().route("/", methods).unwrap()).await;
    let response = client().post(url.clone()).send().await.unwrap();
    assert_eq!(response.status(), 405);
    assert!(response.headers().contains_key("x-scope"));
    let response = client().get(url).send().await.unwrap();
    assert!(!response.headers().contains_key("x-scope"));
    finish(stop, server).await;
}
#[derive(Clone)]
struct Label(String);
#[simple_server::test]
async fn scoped_layers_replay_pending_state_and_keep_nested_captures_and_extensions() {
    let methods = e::MethodRouter::<String>::new()
        .unwrap()
        .on_handler(
            e::Method::GET,
            |e::State(state): e::State<String>,
             e::Extension(label): e::Extension<Label>,
             e::Path(id): e::Path<u32>| async move {
                let mut response =
                    e::Response::new(e::Body::from(format!("{state}:{}:{id}", label.0)));
                response.extensions_mut().insert(Label("response".into()));
                response
            },
        )
        .unwrap()
        .route_layer(m::from_fn(
            |mut request: e::Request, next: m::Next| async move {
                request.extensions_mut().insert(Label("method".into()));
                next.run(request).await
            },
        ))
        .unwrap()
        .layer(m::map_response(|mut response: e::Response| async move {
            assert_eq!(response.extensions().get::<Label>().unwrap().0, "response");
            response
                .headers_mut()
                .append("x-order", "method".parse().unwrap());
            response
        }))
        .unwrap();
    let router = e::Router::<String>::new()
        .unwrap()
        .nest(
            "/api",
            e::Router::new()
                .unwrap()
                .route("/items/{id}", methods)
                .unwrap(),
        )
        .unwrap()
        .route_layer(m::map_response(|mut response: e::Response| async move {
            assert_eq!(response.extensions().get::<Label>().unwrap().0, "response");
            response
                .headers_mut()
                .append("x-order", "router".parse().unwrap());
            response
        }))
        .unwrap();
    for state in ["one", "two"] {
        let (url, stop, server) = start(router.clone().with_state(state.to_owned()).unwrap()).await;
        let response = client()
            .get(format!("{url}/api/items/42"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response
                .headers()
                .get_all("x-order")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["method", "router"]
        );
        assert_eq!(response.text().await.unwrap(), format!("{state}:method:42"));
        finish(stop, server).await;
    }
}
#[cfg(feature = "web")]
#[simple_server::test]
async fn scope_status_headers_and_bodies_match_source_over_http() {
    use simple_server::web as w;
    async fn source_tag(mut response: w::Response) -> w::Response {
        response
            .headers_mut()
            .append("x-scope", "yes".parse().unwrap());
        response
    }
    for scope in 0..3 {
        let methods = w::routing::get(|| async { "get" });
        let methods = match scope {
            1 => methods.layer(w::middleware::map_response(source_tag)),
            2 => methods.route_layer(w::middleware::map_response(source_tag)),
            _ => methods,
        }
        .post(|| async { "post" });
        let source = w::Router::new().route("/existing", methods);
        let source = if scope == 0 {
            source.route_layer(w::middleware::map_response(source_tag))
        } else {
            source
        };
        let source = source.route("/later", w::routing::get(|| async { "later" }));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let source_url = format!("http://{}", listener.local_addr().unwrap());
        let source_stop = simple_server::lifecycle::Shutdown::new();
        let signal = source_stop.clone();
        let source_server = simple_server::runtime::spawn_blocking(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    w::serve(
                        simple_server::net::TcpListener::from_std(listener).unwrap(),
                        source,
                        signal,
                    )
                    .await
                })
        });
        let (url, stop, server) = start(router(scope)).await;
        for (method, path) in [
            ("GET", "/existing"),
            ("HEAD", "/existing"),
            ("POST", "/existing"),
            ("DELETE", "/existing"),
            ("CUSTOM", "/existing"),
            ("GET", "/missing"),
            ("HEAD", "/missing"),
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
            assert_eq!(
                observations[0], observations[1],
                "scope {scope}: {method} {path}"
            );
        }
        finish(stop, server).await;
        source_stop.request();
        timeout(Duration::from_secs(3), source_server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
