#![cfg(all(feature = "engine-web", feature = "client"))]
use simple_server::{
    client::Client,
    engine_web::{self as e, middleware::Layer},
    runtime::spawn,
    time::{sleep, timeout},
};
use std::{
    convert::Infallible,
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tower::{Service, ServiceExt};
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
#[derive(Clone)]
struct CounterLayer(Arc<AtomicUsize>);
#[derive(Clone)]
struct Counter<S> {
    inner: S,
    count: Arc<AtomicUsize>,
}
impl<S> Layer<S> for CounterLayer {
    type Service = Counter<S>;
    fn layer(&self, inner: S) -> Self::Service {
        self.0.fetch_add(1, Ordering::SeqCst);
        Counter {
            inner,
            count: Arc::new(AtomicUsize::new(0)),
        }
    }
}
impl<S, B, R> Service<http::Request<B>> for Counter<S>
where
    S: Service<http::Request<B>, Response = http::Response<R>, Error = Infallible>,
    S::Future: Send + 'static,
    R: Send + 'static,
{
    type Response = http::Response<R>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let count = self.count.fetch_add(1, Ordering::SeqCst) + 1;
        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response = future.await?;
            response
                .headers_mut()
                .insert("x-count", count.to_string().parse().unwrap());
            Ok(response)
        })
    }
}
#[simple_server::test]
async fn layer_created_state_persists_per_route_and_is_not_recreated_per_request() {
    let builds = Arc::new(AtomicUsize::new(0));
    let layer = CounterLayer(builds.clone());
    let router = e::Router::new()
        .unwrap()
        .route(
            "/a",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, |_| async {
                    e::Response::new(e::Body::from("a"))
                })
                .unwrap(),
        )
        .unwrap()
        .route(
            "/b",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, |_| async {
                    e::Response::new(e::Body::from("b"))
                })
                .unwrap(),
        )
        .unwrap()
        .layer(layer)
        .unwrap();
    let baseline = builds.load(Ordering::SeqCst);
    assert!(baseline > 0);
    let (url, stop, server) = start(router).await;
    for (path, count) in [("/a", 1), ("/a", 2), ("/b", 1), ("/a", 3), ("/b", 2)] {
        let response = client().get(format!("{url}{path}")).send().await.unwrap();
        assert_eq!(response.headers()["x-count"], count.to_string());
    }
    assert_eq!(builds.load(Ordering::SeqCst), baseline);
    finish(stop, server).await;
}
#[simple_server::test]
async fn pending_state_validation_does_not_construct_layers_around_placeholders() {
    let builds = Arc::new(AtomicUsize::new(0));
    let router = e::Router::<String>::new()
        .unwrap()
        .route(
            "/",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, |e::State(value): e::State<String>| async {
                    value
                })
                .unwrap(),
        )
        .unwrap()
        .route_layer(CounterLayer(builds.clone()))
        .unwrap();
    assert_eq!(builds.load(Ordering::SeqCst), 0);
    for state in ["one", "two"] {
        let bound = router.clone().with_state(state.to_owned()).unwrap();
        assert!(builds.load(Ordering::SeqCst) > 0);
        let (url, stop, server) = start(bound).await;
        let response = client().get(url).send().await.unwrap();
        assert_eq!(response.headers()["x-count"], "1");
        assert_eq!(response.text().await.unwrap(), state);
        finish(stop, server).await;
    }
}
#[derive(Default)]
struct Gate {
    open: AtomicBool,
    polled: AtomicBool,
    called: AtomicUsize,
    cancelled: AtomicBool,
    waker: futures_util::task::AtomicWaker,
}
#[derive(Clone)]
struct GateLayer(Arc<Gate>);
struct Gated<S> {
    inner: S,
    gate: Arc<Gate>,
    reserved: bool,
    waiting: bool,
}
impl<S> Layer<S> for GateLayer {
    type Service = Gated<S>;
    fn layer(&self, inner: S) -> Self::Service {
        Gated {
            inner,
            gate: self.0.clone(),
            reserved: false,
            waiting: false,
        }
    }
}
impl<S: Clone> Clone for Gated<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            gate: self.gate.clone(),
            reserved: false,
            waiting: false,
        }
    }
}
impl<S> Drop for Gated<S> {
    fn drop(&mut self) {
        if self.waiting {
            self.gate.cancelled.store(true, Ordering::SeqCst);
        }
    }
}
impl<S> Service<e::Request> for Gated<S>
where
    S: Service<e::Request, Response = e::Response, Error = Infallible>,
{
    type Response = e::Response;
    type Error = Infallible;
    type Future = S::Future;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.waiting = true;
        self.gate.polled.store(true, Ordering::SeqCst);
        self.gate.waker.register(cx.waker());
        if !self.gate.open.load(Ordering::SeqCst) {
            return Poll::Pending;
        }
        match self.inner.poll_ready(cx) {
            Poll::Ready(Ok(())) => {
                self.reserved = true;
                self.waiting = false;
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
    fn call(&mut self, request: e::Request) -> Self::Future {
        assert!(self.reserved, "must call the exact ready instance");
        self.reserved = false;
        self.gate.called.fetch_add(1, Ordering::SeqCst);
        self.inner.call(request)
    }
}
async fn until(flag: &AtomicBool) {
    timeout(Duration::from_secs(3), async {
        while !flag.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[simple_server::test]
async fn real_requests_wait_for_readiness_on_the_same_clone_and_cancel_cleanly() {
    let gate = Arc::new(Gate::default());
    let captured = Arc::new(());
    let keep = captured.clone();
    let router = e::Router::<()>::new()
        .unwrap()
        .fallback(move |_| {
            let keep = keep.clone();
            async move {
                drop(keep);
                e::Response::new(e::Body::from("ready"))
            }
        })
        .unwrap()
        .layer(GateLayer(gate.clone()))
        .unwrap();
    let (url, stop, server) = start(router).await;
    let base = url.clone();
    let pending = spawn(async move { client().get(base).send().await });
    until(&gate.polled).await;
    assert_eq!(gate.called.load(Ordering::SeqCst), 0);
    gate.open.store(true, Ordering::SeqCst);
    gate.waker.wake();
    let response = timeout(Duration::from_secs(3), pending)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "ready");
    assert_eq!(gate.called.load(Ordering::SeqCst), 1);
    gate.open.store(false, Ordering::SeqCst);
    gate.polled.store(false, Ordering::SeqCst);
    let pending = spawn(async move { client().get(url).send().await });
    until(&gate.polled).await;
    pending.abort();
    let _ = pending.await;
    until(&gate.cancelled).await;
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
async fn layers_can_replace_the_entire_request_without_losing_the_inner_route() {
    let layer = tower::layer::layer_fn(|inner: e::Route| {
        tower::service_fn(move |_: e::Request| {
            let inner = inner.clone();
            async move {
                let replacement = http::Request::builder()
                    .method("POST")
                    .uri("/replacement")
                    .body(e::Body::from("new body"))
                    .unwrap();
                inner.oneshot(replacement).await
            }
        })
    });
    let router = e::Router::new()
        .unwrap()
        .route(
            "/original/{id}",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, |request| async {
                    let metadata = request.extensions().get::<e::RequestMetadata>().unwrap();
                    assert!(metadata.path_params.is_empty());
                    let description = format!("{} {}", request.method(), request.uri());
                    let body = request.into_body().collect(100).await.unwrap();
                    e::Response::new(e::Body::from(format!(
                        "{description}:{}",
                        String::from_utf8(body.to_vec()).unwrap()
                    )))
                })
                .unwrap(),
        )
        .unwrap()
        .layer(layer)
        .unwrap();
    let (url, stop, server) = start(router).await;
    assert_eq!(
        client()
            .get(format!("{url}/original/42"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "POST /replacement:new body"
    );
    finish(stop, server).await;
}
#[simple_server::test]
async fn third_party_compression_layer_returns_a_different_streaming_body_type() {
    let text = "compressible text ".repeat(200);
    let expected = text.clone();
    let router = e::Router::new()
        .unwrap()
        .route(
            "/",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, move || {
                    let text = text.clone();
                    async move { text }
                })
                .unwrap(),
        )
        .unwrap()
        .layer(tower_http_05::compression::CompressionLayer::new())
        .unwrap();
    let (url, stop, server) = start(router).await;
    let response = client()
        .get(url)
        .header("accept-encoding", "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["content-encoding"], "gzip");
    let bytes = response.bytes().await.unwrap();
    let mut decoder = flate2::read::GzDecoder::new(bytes.as_ref());
    let mut decoded = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut decoded).unwrap();
    assert_eq!(decoded, expected);
    finish(stop, server).await;
}
#[derive(Clone)]
struct PanicLayer;
impl<S> Layer<S> for PanicLayer {
    type Service = S;
    fn layer(&self, _: S) -> S {
        panic!("layer constructor probe")
    }
}
#[simple_server::test]
async fn constructor_panics_return_errors_and_release_owned_route_resources() {
    let captured = Arc::new(());
    let keep = captured.clone();
    let router = e::Router::<()>::new()
        .unwrap()
        .fallback(move |_| {
            let keep = keep.clone();
            async move {
                drop(keep);
                e::Response::new(e::Body::empty())
            }
        })
        .unwrap();
    let router = router.with_state::<()>(()).unwrap();
    assert!(router.layer(PanicLayer).is_err());
    assert_eq!(Arc::strong_count(&captured), 1);
    let router = e::Router::<String>::new()
        .unwrap()
        .route(
            "/",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, |e::State(s): e::State<String>| async { s })
                .unwrap(),
        )
        .unwrap()
        .layer(PanicLayer)
        .unwrap();
    assert!(router.with_state::<()>("state".into()).is_err());
}
#[cfg(feature = "web")]
#[simple_server::test]
async fn stateful_tower_layer_matches_source_responses_and_scope() {
    use simple_server::web as w;
    let source = w::Router::new()
        .route("/", w::routing::get(|| async { "ok" }))
        .layer(CounterLayer(Arc::new(AtomicUsize::new(0))))
        .with_state::<()>(());
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
            "/",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, || async { "ok" })
                .unwrap(),
        )
        .unwrap()
        .layer(CounterLayer(Arc::new(AtomicUsize::new(0))))
        .unwrap();
    let (url, stop, server) = start(engine).await;
    for (method, path) in [
        ("GET", "/"),
        ("GET", "/"),
        ("HEAD", "/"),
        ("POST", "/"),
        ("POST", "/"),
        ("GET", "/missing"),
        ("GET", "/missing"),
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
