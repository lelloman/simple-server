#![cfg(all(feature = "engine-web", feature = "client"))]
use http_body_util::Full;
use simple_server::{
    client::Client,
    engine_web::{self as e, Service},
    runtime::{spawn, spawn_blocking},
    time::{sleep, timeout},
};
use std::{
    convert::Infallible,
    future::{Future, Ready, ready},
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
#[derive(Clone)]
struct Inspect;
impl<B> Service<http::Request<B>> for Inspect {
    type Response = http::Response<Full<e::Bytes>>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let mut response = http::Response::new(Full::new(e::Bytes::from(format!(
            "{} {}",
            request.method(),
            request.uri()
        ))));
        response
            .headers_mut()
            .insert("x-service", "yes".parse().unwrap());
        ready(Ok(response))
    }
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
async fn until(flag: &AtomicBool) {
    timeout(Duration::from_secs(3), async {
        while !flag.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[derive(Default)]
struct Gate {
    open: AtomicBool,
    polled: AtomicBool,
    called: AtomicBool,
    dropped: AtomicBool,
    waker: futures_util::task::AtomicWaker,
}
struct Gated {
    gate: Arc<Gate>,
    ready: bool,
    polled: bool,
    called: bool,
    pending_call: bool,
}
impl Clone for Gated {
    fn clone(&self) -> Self {
        Self {
            gate: self.gate.clone(),
            ready: false,
            polled: false,
            called: false,
            pending_call: self.pending_call,
        }
    }
}
impl Drop for Gated {
    fn drop(&mut self) {
        if self.polled && !self.called {
            self.gate.dropped.store(true, Ordering::SeqCst);
        }
    }
}
struct CallGuard(Arc<Gate>);
impl Drop for CallGuard {
    fn drop(&mut self) {
        self.0.dropped.store(true, Ordering::SeqCst);
    }
}
impl Service<e::Request> for Gated {
    type Response = http::Response<Full<e::Bytes>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.polled = true;
        self.gate.polled.store(true, Ordering::SeqCst);
        self.gate.waker.register(cx.waker());
        if self.gate.open.load(Ordering::SeqCst) {
            self.ready = true;
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn call(&mut self, _: e::Request) -> Self::Future {
        assert!(
            self.ready,
            "service must be called on the clone whose readiness was polled"
        );
        self.called = true;
        self.gate.called.store(true, Ordering::SeqCst);
        let pending = self.pending_call;
        let guard = CallGuard(self.gate.clone());
        Box::pin(async move {
            let _guard = guard;
            if pending {
                std::future::pending::<()>().await;
            }
            Ok(http::Response::new(Full::new(e::Bytes::from_static(
                b"ready",
            ))))
        })
    }
}
fn gated(gate: Arc<Gate>, pending_call: bool) -> Gated {
    Gated {
        gate,
        ready: false,
        polled: false,
        called: false,
        pending_call,
    }
}
#[simple_server::test]
async fn service_readiness_is_awaited_on_the_same_clone_before_call() {
    for nested in [false, true] {
        let gate = Arc::new(Gate::default());
        let router = e::Router::new()
            .unwrap()
            .route(
                "/",
                e::MethodRouter::any_service(gated(gate.clone(), false)).unwrap(),
            )
            .unwrap();
        let router = if nested {
            e::Router::new()
                .unwrap()
                .nest_service("/mounted", gated(gate.clone(), false))
                .unwrap()
        } else {
            router
        };
        assert!(!gate.polled.load(Ordering::SeqCst));
        let (url, stop, server) = start(router).await;
        let response = spawn(async move {
            Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .get(if nested {
                    format!("{url}/mounted")
                } else {
                    url
                })
                .send()
                .await
                .unwrap()
        });
        until(&gate.polled).await;
        assert!(!gate.called.load(Ordering::SeqCst));
        gate.open.store(true, Ordering::SeqCst);
        gate.waker.wake();
        assert_eq!(
            timeout(Duration::from_secs(3), response)
                .await
                .unwrap()
                .unwrap()
                .text()
                .await
                .unwrap(),
            "ready"
        );
        finish(stop, server).await;
    }
}
#[simple_server::test]
async fn disconnect_drops_waiting_readiness_and_pending_service_calls() {
    for (nested, pending_call) in [(false, false), (false, true), (true, false), (true, true)] {
        let gate = Arc::new(Gate::default());
        gate.open.store(pending_call, Ordering::SeqCst);
        let router = e::Router::new().unwrap();
        let router = if nested {
            router.nest_service("/mounted", gated(gate.clone(), pending_call))
        } else {
            router.fallback_service(gated(gate.clone(), pending_call))
        }
        .unwrap();
        let (url, stop, server) = start(router).await;
        let socket = spawn_blocking(move || {
            use std::io::Write;
            let mut socket =
                std::net::TcpStream::connect(url.trim_start_matches("http://")).unwrap();
            socket
                .write_all(b"GET /mounted HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
            socket
        })
        .await
        .unwrap();
        until(if pending_call {
            &gate.called
        } else {
            &gate.polled
        })
        .await;
        drop(socket);
        until(&gate.dropped).await;
        if !pending_call {
            assert!(!gate.called.load(Ordering::SeqCst));
        }
        finish(stop, server).await;
    }
}
#[derive(Clone)]
struct Echo;
impl Service<e::Request> for Echo {
    type Response = e::Response;
    type Error = Infallible;
    type Future = Ready<Result<e::Response, Infallible>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: e::Request) -> Self::Future {
        assert!(request.extensions().get::<e::RequestMetadata>().is_some());
        ready(Ok(e::Response::new(request.into_body())))
    }
}
#[simple_server::test]
async fn service_response_can_stream_the_incoming_body_after_call_returns() {
    for nested in [false, true] {
        let router = e::Router::new()
            .unwrap()
            .route(
                "/echo",
                e::MethodRouter::new()
                    .unwrap()
                    .on_service(e::Method::POST, Echo)
                    .unwrap(),
            )
            .unwrap();
        let router = if nested {
            e::Router::new()
                .unwrap()
                .nest_service("/echo", Echo)
                .unwrap()
        } else {
            router
        };
        let listener = e::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr();
        let stop = e::Shutdown::new();
        let server = spawn(e::serve(listener, router, stop.clone()));
        let output=spawn_blocking(move|| {
        use std::io::{Read,Write};let mut socket=std::net::TcpStream::connect(address).unwrap();socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        socket.write_all(b"POST /echo HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nfirst\r\n").unwrap();
        let mut output=Vec::new();let mut bytes=[0;1024];
        while !output.windows(5).any(|w|w==b"first"){let n=socket.read(&mut bytes).unwrap();assert!(n>0);output.extend_from_slice(&bytes[..n]);}
        // Receiving the first chunk before sending the second proves there was no collection.
        socket.write_all(b"4\r\nlast\r\n0\r\n\r\n").unwrap();socket.read_to_end(&mut output).unwrap();output
    }).await.unwrap();
        assert!(output.windows(4).any(|w| w == b"last"));
        finish(stop, server).await;
    }
}
#[simple_server::test]
async fn services_compose_with_pending_typed_state_and_method_fallbacks() {
    async fn state(e::State(value): e::State<String>) -> String {
        value
    }
    let methods = e::MethodRouter::new()
        .unwrap()
        .on_handler(e::Method::POST, state)
        .unwrap()
        .on_service(e::Method::GET, Inspect)
        .unwrap();
    let router = e::Router::new()
        .unwrap()
        .route("/mixed", methods)
        .unwrap()
        .fallback_methods(
            e::MethodRouter::new()
                .unwrap()
                .on_service(e::Method::GET, Inspect)
                .unwrap(),
        )
        .unwrap()
        .with_state("state".to_owned())
        .unwrap();
    let (url, stop, server) = start(router).await;
    let client = Client::builder().no_proxy().build().unwrap();
    assert_eq!(
        client
            .post(format!("{url}/mixed"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "state"
    );
    assert_eq!(
        client
            .get(format!("{url}/mixed?x=1"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "GET /mixed?x=1"
    );
    assert_eq!(
        client
            .post(format!("{url}/missing"))
            .send()
            .await
            .unwrap()
            .status(),
        e::StatusCode::METHOD_NOT_ALLOWED
    );
    finish(stop, server).await;
}
#[cfg(all(feature = "web", feature = "lifecycle"))]
#[simple_server::test]
async fn method_and_fallback_services_match_source_over_http() {
    use simple_server::web as w;
    let source = w::Router::new()
        .route("/service", w::routing::get_service(Inspect))
        .fallback_service(Inspect)
        .nest_service("/svc", Inspect)
        .nest_service("/slash/", Inspect)
        .nest(
            "/org/{org}",
            w::Router::new().nest_service("/files", Inspect),
        );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let source_url = format!("http://{}", listener.local_addr().unwrap());
    let source_stop = simple_server::lifecycle::Shutdown::new();
    let stop_source = source_stop.clone();
    let source_server = spawn_blocking(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                w::serve(
                    simple_server::net::TcpListener::from_std(listener).unwrap(),
                    source,
                    stop_source,
                )
                .await
            })
    });
    let engine = e::Router::new()
        .unwrap()
        .route(
            "/service",
            e::MethodRouter::new()
                .unwrap()
                .on_service(e::Method::GET, Inspect)
                .unwrap(),
        )
        .unwrap()
        .fallback_service(Inspect)
        .unwrap()
        .nest_service("/svc", Inspect)
        .unwrap()
        .nest_service("/slash/", Inspect)
        .unwrap()
        .nest(
            "/org/{org}",
            e::Router::new()
                .unwrap()
                .nest_service("/files", Inspect)
                .unwrap(),
        )
        .unwrap();
    let (url, stop, server) = start(engine).await;
    let client = Client::builder().no_proxy().build().unwrap();
    for (method, path) in [
        ("GET", "/svc"),
        ("GET", "/svc/"),
        ("GET", "/svc/a%2Fb%252Fc?x=a%2Fb"),
        ("HEAD", "/svc/a"),
        ("POST", "/svc/a"),
        ("CUSTOM", "/svc/a"),
        ("GET", "/svcx"),
        ("GET", "/slash"),
        ("GET", "/slash/"),
        ("GET", "/slash/a?q=1"),
        ("GET", "/org/acme/files/a?q=1"),
        ("GET", "/service?x=1"),
        ("HEAD", "/service"),
        ("POST", "/service"),
        ("GET", "/missing?x=1"),
        ("PATCH", "/missing"),
        ("HEAD", "/missing"),
    ] {
        let response = client
            .request(method.parse().unwrap(), format!("{url}{path}"))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let mut headers = response.headers().clone();
        headers.remove("date");
        let body = response.bytes().await.unwrap().to_vec();
        let expected = client
            .request(method.parse().unwrap(), format!("{source_url}{path}"))
            .send()
            .await
            .unwrap();
        let expected_status = expected.status();
        let mut expected_headers = expected.headers().clone();
        expected_headers.remove("date");
        assert_eq!(
            (status, headers, body),
            (
                expected_status,
                expected_headers,
                expected.bytes().await.unwrap().to_vec()
            ),
            "{method} {path}"
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

#[simple_server::test]
async fn nested_services_preserve_pending_state_and_validate_before_binding() {
    async fn state(e::State(value): e::State<String>) -> String {
        value
    }
    let router = e::Router::new()
        .unwrap()
        .route(
            "/state",
            e::MethodRouter::new()
                .unwrap()
                .on_handler(e::Method::GET, state)
                .unwrap(),
        )
        .unwrap()
        .nest_service("/svc", Inspect)
        .unwrap();
    assert!(router.clone().nest_service("no-slash", Inspect).is_err());
    assert!(router.clone().nest_service("/", Inspect).is_err());
    assert!(
        router
            .clone()
            .nest_service("/wild/{*rest}", Inspect)
            .is_err()
    );
    assert!(router.clone().nest_service("/svc", Inspect).is_err());
    let client = Client::builder().no_proxy().build().unwrap();
    for value in ["first", "second"] {
        let bound = router
            .clone()
            .with_state(value.to_owned())
            .unwrap()
            .merge(
                e::Router::new()
                    .unwrap()
                    .nest_service("/other", Inspect)
                    .unwrap(),
            )
            .unwrap();
        let (url, stop, server) = start(bound).await;
        for (path, expected) in [
            ("/state", value),
            ("/svc/a?q=1", "GET /a?q=1"),
            ("/other", "GET /"),
        ] {
            assert_eq!(
                client
                    .get(format!("{url}{path}"))
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap(),
                expected
            );
        }
        assert_eq!(
            client
                .get(format!("{url}/svcx"))
                .send()
                .await
                .unwrap()
                .status(),
            e::StatusCode::NOT_FOUND
        );
        finish(stop, server).await;
    }
}
