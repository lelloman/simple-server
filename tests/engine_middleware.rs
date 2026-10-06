#![cfg(all(feature = "engine-web", feature = "client"))]
use http_body_util::BodyExt;
use simple_server::{
    client::Client,
    engine_web::{
        self as e,
        middleware::{self as m, Layer},
    },
    runtime::spawn,
};
use std::{
    convert::Infallible,
    future::{Ready, ready},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use tower::ServiceExt;

#[derive(Clone, Debug)]
struct Marker(String);

#[simple_server::test]
async fn state_extensions_and_response_mapping_survive_real_routing() {
    let handler = m::handler_service(
        |e::State(prefix): e::State<String>,
         e::Extension(marker): e::Extension<Marker>,
         e::Path(id): e::Path<u64>| async move {
            let mut response =
                e::Response::new(e::Body::from(format!("{prefix}:{}:{id}", marker.0)));
            response.extensions_mut().insert(Marker("response".into()));
            response
        },
        "handler".to_owned(),
    );
    let inner = m::from_fn_with_state(
        "middleware".to_owned(),
        |e::State(value): e::State<String>, mut request: e::Request, next: m::Next| async move {
            assert_eq!(
                request
                    .extensions()
                    .get::<e::RequestMetadata>()
                    .unwrap()
                    .matched_path
                    .as_deref(),
                Some("/api/{id}")
            );
            request.extensions_mut().insert(Marker(value));
            let mut response = next.run(request).await;
            assert_eq!(response.extensions().get::<Marker>().unwrap().0, "response");
            response
                .headers_mut()
                .append("x-order", "inner".parse().unwrap());
            response
        },
    )
    .layer(handler);
    let service = m::map_response(|mut response: e::Response| async move {
        assert_eq!(response.extensions().get::<Marker>().unwrap().0, "response");
        response
            .headers_mut()
            .append("x-order", "outer".parse().unwrap());
        response
    })
    .layer(inner);
    let router = e::Router::new()
        .unwrap()
        .route(
            "/api/{id}",
            e::MethodRouter::new()
                .unwrap()
                .on_service(e::Method::GET, service)
                .unwrap(),
        )
        .unwrap();
    let listener = e::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr());
    let stop = e::Shutdown::new();
    let server = spawn(e::serve(listener, router, stop.clone()));
    let client = Client::builder().no_proxy().build().unwrap();
    let response = client.get(format!("{url}/api/42")).send().await.unwrap();
    assert_eq!(
        response
            .headers()
            .get_all("x-order")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect::<Vec<_>>(),
        ["inner", "outer"]
    );
    assert_eq!(
        response.bytes().await.unwrap().as_ref(),
        b"handler:middleware:42"
    );
    // A service-local layer must not pretend to intercept native 404/405 replies.
    for (path, method, status) in [
        ("/api/42", e::Method::POST, 405),
        ("/missing", e::Method::GET, 404),
    ] {
        let response = client
            .request(method, format!("{url}{path}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        assert!(!response.headers().contains_key("x-order"));
    }
    stop.request();
    simple_server::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[derive(Default)]
struct Gate {
    open: AtomicBool,
    polls: AtomicUsize,
    calls: AtomicUsize,
    cancelled: AtomicBool,
    waker: futures_util::task::AtomicWaker,
}
struct Reserved {
    gate: Arc<Gate>,
    ready: bool,
    polled: bool,
}
impl Clone for Reserved {
    fn clone(&self) -> Self {
        Self {
            gate: self.gate.clone(),
            ready: false,
            polled: false,
        }
    }
}
impl Drop for Reserved {
    fn drop(&mut self) {
        if self.polled && !self.ready {
            self.gate.cancelled.store(true, Ordering::SeqCst);
        }
    }
}
impl e::Service<e::Request> for Reserved {
    type Response = e::Response;
    type Error = Infallible;
    type Future = Ready<Result<e::Response, Infallible>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.polled = true;
        self.gate.polls.fetch_add(1, Ordering::SeqCst);
        self.gate.waker.register(cx.waker());
        if self.gate.open.load(Ordering::SeqCst) {
            self.ready = true;
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn call(&mut self, request: e::Request) -> Self::Future {
        assert!(self.ready, "call must use the exact ready clone");
        self.ready = false;
        self.polled = false;
        self.gate.calls.fetch_add(1, Ordering::SeqCst);
        ready(Ok(e::Response::new(request.into_body())))
    }
}
fn reserved(gate: &Arc<Gate>) -> Reserved {
    Reserved {
        gate: gate.clone(),
        ready: false,
        polled: false,
    }
}
async fn pass(request: e::Request, next: m::Next) -> e::Response {
    next.run(request).await
}

#[simple_server::test]
async fn readiness_is_waited_on_the_same_clone_and_cancellation_releases_it() {
    let gate = Arc::new(Gate::default());
    let service = m::from_fn(pass).layer(reserved(&gate));
    let mut future = Box::pin(service.oneshot(e::Request::new(e::Body::from("hello"))));
    assert!(futures_util::poll!(&mut future).is_pending());
    assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
    gate.open.store(true, Ordering::SeqCst);
    gate.waker.wake();
    assert_eq!(
        future
            .await
            .unwrap()
            .into_body()
            .collect(100)
            .await
            .unwrap()
            .as_ref(),
        b"hello"
    );
    assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    assert!(!gate.cancelled.load(Ordering::SeqCst));
    gate.open.store(false, Ordering::SeqCst);
    let mut future = Box::pin(
        m::from_fn(pass)
            .layer(reserved(&gate))
            .oneshot(e::Request::new(e::Body::empty())),
    );
    assert!(futures_util::poll!(&mut future).is_pending());
    drop(future);
    assert!(gate.cancelled.load(Ordering::SeqCst));
}

#[simple_server::test]
async fn short_circuit_and_extractor_rejection_do_not_poll_inner_service() {
    let gate = Arc::new(Gate::default());
    let denied = m::from_fn(|_: e::Request, _: m::Next| async { e::StatusCode::FORBIDDEN })
        .layer(reserved(&gate));
    assert_eq!(
        denied
            .oneshot(e::Request::new(e::Body::empty()))
            .await
            .unwrap()
            .status(),
        403
    );
    let needs_extension = m::from_fn(
        |_: e::Extension<Marker>, request: e::Request, next: m::Next| async move {
            next.run(request).await
        },
    )
    .layer(reserved(&gate));
    assert_eq!(
        needs_extension
            .oneshot(e::Request::new(e::Body::empty()))
            .await
            .unwrap()
            .status(),
        500
    );
    assert_eq!(gate.polls.load(Ordering::SeqCst), 0);
    assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
}

#[simple_server::test]
async fn response_mapping_includes_handler_extraction_errors() {
    let service = m::map_response(|mut response: e::Response| async move {
        response
            .headers_mut()
            .insert("x-mapped", "yes".parse().unwrap());
        response
    })
    .layer(m::handler_service(
        |e::Json(_): e::Json<serde_json::Value>| async { "unexpected" },
        (),
    ));
    let request = http::Request::builder()
        .header("content-type", "application/json")
        .body(e::Body::from("{"))
        .unwrap();
    let response = service.oneshot(request).await.unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(response.headers()["x-mapped"], "yes");
}

#[simple_server::test]
async fn middleware_keeps_bodies_lazy_and_preserves_trailers_and_late_errors() {
    let polls = Arc::new(AtomicUsize::new(0));
    let observed = polls.clone();
    let mut trailers = e::HeaderMap::new();
    trailers.insert("x-end", "done".parse().unwrap());
    use futures_util::StreamExt;
    let stream = futures_util::stream::iter([
        Ok::<_, std::io::Error>(e::Frame::data(e::Bytes::from_static(b"first"))),
        Ok(e::Frame::trailers(trailers)),
    ])
    .inspect(move |_| {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    let service = m::from_fn(pass).layer(m::handler_service(
        |request: e::Request| async { e::Response::new(request.into_body()) },
        (),
    ));
    let response = service
        .clone()
        .oneshot(e::Request::new(e::Body::new(
            http_body_util::StreamBody::new(stream),
        )))
        .await
        .unwrap();
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let collected = BodyExt::collect(response.into_body()).await.unwrap();
    assert_eq!(collected.trailers().unwrap()["x-end"], "done");
    assert_eq!(collected.to_bytes(), "first");
    let body = e::Body::from_stream(futures_util::stream::iter([
        Ok(e::Bytes::from_static(b"first")),
        Err(std::io::Error::other("late failure")),
    ]));
    let mut response = service.oneshot(e::Request::new(body)).await.unwrap();
    assert_eq!(
        response
            .body_mut()
            .frame()
            .await
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap(),
        "first"
    );
    assert!(response.body_mut().frame().await.unwrap().is_err());
}

#[cfg(feature = "web")]
#[simple_server::test]
async fn host_middleware_contract_matches_source_backend() {
    macro_rules! contract {
        ($web:path, $middleware:path) => {{
            use $web as w;
            use $middleware as mw;
            let service = tower::service_fn(|request: w::Request| async move {
                let marker = request.extensions().get::<Marker>().unwrap().0.clone();
                let mut response = w::Response::new(request.into_body());
                response.extensions_mut().insert(Marker(marker));
                Ok::<_, Infallible>(response)
            });
            let service = mw::from_fn_with_state("value".to_owned(), |w::State(value): w::State<String>, mut request: w::Request, next: mw::Next| async move {
                request.extensions_mut().insert(Marker(value));
                let mut response = next.run(request).await;
                let marker = response.extensions().get::<Marker>().unwrap().0.clone();
                response.headers_mut().insert("x-marker", marker.parse().unwrap());
                response
            }).layer(service);
            let response = service.oneshot(w::Request::new(w::Body::from("stream"))).await.unwrap();
            let (parts, body) = response.into_parts();
            (parts.status, parts.headers, body.collect(100).await.unwrap())
        }};
    }
    assert_eq!(
        contract!(
            simple_server::engine_web,
            simple_server::engine_web::middleware
        ),
        contract!(simple_server::web, simple_server::web::middleware)
    );
}

struct BodyProbe(Arc<AtomicBool>);
impl Drop for BodyProbe {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
impl futures_util::Stream for BodyProbe {
    type Item = Result<e::Bytes, std::io::Error>;
    fn poll_next(self: std::pin::Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}
#[simple_server::test]
async fn dropping_response_or_waiting_middleware_releases_body_producers() {
    let dropped = Arc::new(AtomicBool::new(false));
    let service = m::from_fn(pass).layer(m::handler_service(
        |request: e::Request| async { e::Response::new(request.into_body()) },
        (),
    ));
    let response = service
        .oneshot(e::Request::new(e::Body::from_stream(BodyProbe(
            dropped.clone(),
        ))))
        .await
        .unwrap();
    assert!(!dropped.load(Ordering::SeqCst));
    drop(response);
    assert!(dropped.load(Ordering::SeqCst));

    let dropped = Arc::new(AtomicBool::new(false));
    let gate = Arc::new(Gate::default());
    let service = m::from_fn(pass).layer(reserved(&gate));
    let mut future = Box::pin(
        service.oneshot(e::Request::new(e::Body::from_stream(BodyProbe(
            dropped.clone(),
        )))),
    );
    assert!(futures_util::poll!(&mut future).is_pending());
    assert!(!dropped.load(Ordering::SeqCst));
    drop(future);
    assert!(dropped.load(Ordering::SeqCst));
    assert!(gate.cancelled.load(Ordering::SeqCst));
}
