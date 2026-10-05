#![cfg(all(feature = "engine-web", feature = "client"))]
use simple_server::{
    client::Client,
    engine_web::{self as e, Method, MethodRouter, Router, State},
    runtime::spawn,
    time::timeout,
};
use std::{io, sync::Arc, time::Duration};
#[derive(Clone)]
struct App(Arc<String>);
struct Name(String);
impl e::FromState<App> for Name {
    fn from_state(state: &App) -> Self {
        Self(state.0.as_ref().clone())
    }
}
#[cfg(feature = "web")]
impl simple_server::web::FromState<App> for Name {
    fn from_state(state: &App) -> Self {
        Self(state.0.as_ref().clone())
    }
}
async fn shared(State(Name(name)): State<Name>) -> String {
    name
}
async fn local(State(name): State<String>) -> String {
    name
}
fn app() -> io::Result<Router<App>> {
    let mixed = MethodRouter::<String>::new()?
        .on_handler(Method::GET, local)?
        .with_state::<App>("method".into())?
        .on_handler(Method::POST, shared)?;
    let child = Router::<App>::new()?
        .route(
            "/read",
            MethodRouter::new()?.on_handler(Method::GET, shared)?,
        )?
        .with_state::<App>(App(Arc::new("child".into())))?;
    Router::new()?
        .route(
            "/shared",
            MethodRouter::new()?.on_handler(Method::GET, shared)?,
        )?
        .route("/mixed", mixed)?
        .nest("/child", child)?
        .merge(Router::new()?.route(
            "/merged",
            MethodRouter::new()?.on_handler(Method::GET, shared)?,
        )?)?
        .route(
            "/raw",
            MethodRouter::new()?.on(Method::GET, |_| async {
                e::Response::new(e::Body::from("raw"))
            })?,
        )?
        .fallback_handler(shared)
}
async fn start(
    router: Router,
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
    timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
async fn fetch(
    client: &Client,
    base: &str,
    method: &str,
    path: &str,
) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
    let response = timeout(
        Duration::from_secs(5),
        client
            .request(method.parse().unwrap(), format!("{base}{path}"))
            .send(),
    )
    .await
    .unwrap()
    .unwrap();
    let status = response.status();
    let mut headers = response.headers().clone();
    headers.remove("date");
    (status, headers, response.bytes().await.unwrap().to_vec())
}
#[simple_server::test]
async fn cloned_pending_routers_bind_independently_and_preserve_local_state() {
    let pending = app().unwrap();
    let (a, sa, ja) = start(
        pending
            .clone()
            .with_state(App(Arc::new("first".into())))
            .unwrap(),
    )
    .await;
    let (b, sb, jb) = start(pending.with_state(App(Arc::new("second".into()))).unwrap()).await;
    let client = Client::builder().no_proxy().build().unwrap();
    for (base, outer) in [(&a, "first"), (&b, "second")] {
        for (method, path, body) in [
            ("GET", "/shared", outer),
            ("GET", "/mixed", "method"),
            ("POST", "/mixed", outer),
            ("GET", "/child/read", "child"),
            ("GET", "/merged", outer),
            ("GET", "/raw", "raw"),
            ("GET", "/missing", outer),
        ] {
            let (status, _, bytes) = fetch(&client, base, method, path).await;
            assert_eq!(status, e::StatusCode::OK);
            assert_eq!(bytes, body.as_bytes());
        }
    }
    finish(sa, ja).await;
    finish(sb, jb).await;
}
#[simple_server::test]
async fn state_can_be_bound_in_stages_and_method_fallbacks_keep_method_semantics() {
    let router = Router::<App>::new()
        .unwrap()
        .route(
            "/first",
            MethodRouter::new()
                .unwrap()
                .on_handler(Method::GET, shared)
                .unwrap(),
        )
        .unwrap()
        .with_state::<String>(App(Arc::new("first".into())))
        .unwrap()
        .route("/later", MethodRouter::any_handler(local).unwrap())
        .unwrap()
        .fallback_methods(
            MethodRouter::new()
                .unwrap()
                .on_handler(Method::GET, local)
                .unwrap(),
        )
        .unwrap()
        .with_state("later".into())
        .unwrap();
    let (base, stop, server) = start(router).await;
    let client = Client::builder().no_proxy().build().unwrap();
    for (method, path, status, body) in [
        ("GET", "/first", 200, "first"),
        ("PATCH", "/later", 200, "later"),
        ("GET", "/missing", 200, "later"),
        ("HEAD", "/missing", 200, ""),
        ("POST", "/missing", 405, ""),
    ] {
        let (actual, headers, bytes) = fetch(&client, &base, method, path).await;
        assert_eq!(actual.as_u16(), status);
        assert_eq!(bytes, body.as_bytes());
        if status == 405 {
            assert!(headers["allow"].to_str().unwrap().contains("GET"));
        }
    }
    finish(stop, server).await;
}
#[simple_server::test]
async fn raw_and_handler_local_bindings_work_in_a_pending_method_group() {
    let methods = MethodRouter::<App>::new()
        .unwrap()
        .on_handler(Method::GET, shared)
        .unwrap()
        .on_state(Method::POST, "local".to_owned(), local)
        .unwrap()
        .on(Method::PUT, |_| async {
            e::Response::new(e::Body::from("raw"))
        })
        .unwrap();
    let router = Router::new()
        .unwrap()
        .route("/", methods)
        .unwrap()
        .fallback(|_| async { e::Response::new(e::Body::from("fallback")) })
        .unwrap()
        .with_state(App(Arc::new("outer".into())))
        .unwrap();
    let (base, stop, server) = start(router).await;
    let client = Client::builder().no_proxy().build().unwrap();
    for (method, path, expected) in [
        ("GET", "/", "outer"),
        ("POST", "/", "local"),
        ("PUT", "/", "raw"),
        ("GET", "/missing", "fallback"),
    ] {
        assert_eq!(
            fetch(&client, &base, method, path).await.2,
            expected.as_bytes()
        );
    }
    finish(stop, server).await;
}
#[simple_server::test]
async fn invalid_pending_routes_reject_eagerly_and_bound_state_is_released() {
    let methods = MethodRouter::<App>::new()
        .unwrap()
        .on_handler(Method::GET, shared)
        .unwrap();
    assert!(
        Router::<App>::new()
            .unwrap()
            .route("invalid", methods.clone())
            .is_err()
    );
    assert!(methods.clone().on_handler(Method::GET, shared).is_err());
    assert!(
        MethodRouter::<App>::new()
            .unwrap()
            .on_handler(Method::from_bytes(b"CUSTOM").unwrap(), shared)
            .is_err()
    );
    let pending = Router::new().unwrap().route("/", methods).unwrap();
    let value = Arc::new("owned".to_owned());
    let weak = Arc::downgrade(&value);
    let bound: Router = pending.with_state(App(value)).unwrap();
    let copy = bound.clone();
    drop(bound);
    assert!(weak.upgrade().is_some());
    drop(copy);
    assert!(weak.upgrade().is_none());
}
#[cfg(feature = "web")]
#[simple_server::test]
async fn state_composition_matches_source_status_headers_and_body() {
    use simple_server::web as w;
    use tower::ServiceExt;
    async fn shared(w::State(Name(name)): w::State<Name>) -> String {
        name
    }
    async fn local(w::State(name): w::State<String>) -> String {
        name
    }
    let mixed = w::routing::get(local)
        .with_state::<App>("method".into())
        .post(shared);
    let child = w::Router::new()
        .route("/read", w::routing::get(shared))
        .with_state::<App>(App(Arc::new("child".into())));
    let source = w::Router::new()
        .route("/shared", w::routing::get(shared))
        .route("/mixed", mixed)
        .nest("/child", child)
        .merge(w::Router::new().route("/merged", w::routing::get(shared)))
        .route(
            "/raw",
            w::routing::get(|| async { w::Response::new(w::Body::from("raw")) }),
        )
        .fallback(shared)
        .with_state(App(Arc::new("outer".into())));
    let (base, stop, server) = start(
        app()
            .unwrap()
            .with_state(App(Arc::new("outer".into())))
            .unwrap(),
    )
    .await;
    let client = Client::builder().no_proxy().build().unwrap();
    for (method, path) in [
        ("GET", "/shared"),
        ("HEAD", "/shared"),
        ("POST", "/shared"),
        ("GET", "/mixed"),
        ("POST", "/mixed"),
        ("PUT", "/mixed"),
        ("GET", "/child/read"),
        ("GET", "/child/missing"),
        ("GET", "/merged"),
        ("GET", "/raw"),
        ("GET", "/missing"),
    ] {
        let expected = source
            .clone()
            .oneshot(
                w::Request::builder()
                    .method(method)
                    .uri(path)
                    .body(w::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (head, body) = expected.into_parts();
        assert_eq!(
            fetch(&client, &base, method, path).await,
            (
                head.status,
                head.headers,
                body.collect(1024).await.unwrap().to_vec()
            ),
            "{method} {path}"
        );
    }
    finish(stop, server).await;
}
