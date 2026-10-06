#![cfg(all(
    feature = "engine-tokio",
    feature = "engine-tracing",
    feature = "correlation-core",
    feature = "client"
))]
use simple_server::{engine_web as e, runtime::Runtime};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tracing::instrument::WithSubscriber;
#[derive(Clone)]
struct Observer(Arc<AtomicUsize>);
impl e::tracing::Observer for Observer {
    fn on_response(
        &mut self,
        _: &tracing::Span,
        response: &e::tracing::ResponseInfo<'_>,
        _: Duration,
    ) {
        assert_eq!(response.status(), e::StatusCode::OK);
        assert_eq!(
            simple_server::correlation::current_header_id()
                .unwrap()
                .as_header_value(),
            "selected"
        );
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[tokio::test]
async fn host_runtime_correlation_tracing_and_lazy_body_survive_engine_callbacks() {
    let runtime = Runtime::new().unwrap();
    assert!(simple_server::runtime::Handle::try_current().is_err());
    runtime
        .scope(async {
            let observations = Arc::new(AtomicUsize::new(0));
            let observed = observations.clone();
            let router = e::Router::new()
                .unwrap()
                .route(
                    "/stream/{id}",
                    e::MethodRouter::new()
                        .unwrap()
                        .on_handler(e::Method::GET, || async {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            assert_eq!(
                                tracing::Span::current().metadata().unwrap().name(),
                                "http.request"
                            );
                            assert_eq!(
                                simple_server::correlation::current_header_id()
                                    .unwrap()
                                    .as_header_value(),
                                "selected"
                            );
                            e::Body::from_stream(futures_util::stream::once(async {
                                tokio::time::sleep(Duration::from_millis(1)).await;
                                Ok::<_, std::io::Error>("body")
                            }))
                        })
                        .unwrap(),
                )
                .unwrap()
                .layer(e::middleware::from_fn(
                    move |request: e::Request, next: e::middleware::Next| {
                        let observer = Observer(observed.clone());
                        async move {
                            assert_eq!(
                                request
                                    .extensions()
                                    .get::<e::extract::MatchedPath>()
                                    .unwrap()
                                    .as_str(),
                                "/stream/{id}"
                            );
                            e::tracing::trace_with_observer(request, observer, |request| {
                                next.run(request)
                            })
                            .await
                        }
                    },
                ))
                .unwrap()
                .layer(e::middleware::from_fn(
                    |request: e::Request, next: e::middleware::Next| async {
                        use simple_server::correlation::*;
                        Correlation::default()
                            .run_selected(
                                request,
                                HeaderRequestId::new("selected".parse().unwrap()),
                                Propagation::default(),
                                |request| next.run(request),
                            )
                            .await
                    },
                ))
                .unwrap();
            let listener = e::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/stream/private", listener.local_addr());
            let stop = e::Shutdown::new();
            let server = tokio::spawn(Runtime::scope(
                &runtime,
                e::serve(listener, router, stop.clone()),
            ));
            let response = simple_server::client::Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .get(url)
                .send()
                .await
                .unwrap();
            assert_eq!(response.headers()["x-request-id"], "selected");
            assert_eq!(response.text().await.unwrap(), "body");
            assert_eq!(observations.load(Ordering::SeqCst), 1);
            assert!(simple_server::correlation::current_header_id().is_none());
            stop.request();
            tokio::time::timeout(Duration::from_secs(3), server)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        })
        .with_subscriber(tracing_subscriber::fmt().with_test_writer().finish())
        .await;
    assert!(simple_server::runtime::Handle::try_current().is_err());
}

#[tokio::test]
async fn external_executor_scope_restores_context_on_pending_and_cancellation() {
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };
    struct Pending(Arc<AtomicUsize>);
    impl Future for Pending {
        type Output = ();
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            assert!(simple_server::runtime::Handle::try_current().is_ok());
            self.0.fetch_add(1, Ordering::SeqCst);
            Poll::Pending
        }
    }
    impl Drop for Pending {
        fn drop(&mut self) {
            assert!(simple_server::runtime::Handle::try_current().is_ok());
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let runtime = Runtime::new().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let future = runtime.scope(Pending(count.clone()));
    assert!(
        tokio::time::timeout(Duration::from_millis(5), future)
            .await
            .is_err()
    );
    assert!(count.load(Ordering::SeqCst) >= 2);
    assert!(simple_server::runtime::Handle::try_current().is_err());
}

#[tokio::test]
async fn native_static_fallback_keeps_ranges_and_reserved_routes() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("app.js"), "0123456789").unwrap();
    let runtime = Runtime::new().unwrap();
    runtime
        .scope(async {
            let router = e::Router::new()
                .unwrap()
                .route(
                    "/api/{*path}",
                    e::MethodRouter::any_handler(|| async { e::StatusCode::NOT_FOUND }).unwrap(),
                )
                .unwrap()
                .fallback_static_dir(directory.path())
                .unwrap();
            let listener = e::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr());
            let stop = e::Shutdown::new();
            let server = tokio::spawn(runtime.scope(e::serve(listener, router, stop.clone())));
            let client = simple_server::client::Client::builder()
                .no_proxy()
                .build()
                .unwrap();
            let response = client
                .get(format!("{url}/app.js"))
                .header("range", "bytes=2-5")
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), e::StatusCode::PARTIAL_CONTENT);
            assert_eq!(response.headers()["content-range"], "bytes 2-5/10");
            assert_eq!(response.text().await.unwrap(), "2345");
            assert_eq!(
                client
                    .get(format!("{url}/api/app.js"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                e::StatusCode::NOT_FOUND
            );
            assert_eq!(
                client
                    .post(format!("{url}/app.js"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                e::StatusCode::METHOD_NOT_ALLOWED
            );
            stop.request();
            tokio::time::timeout(Duration::from_secs(3), server)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        })
        .await;
}

#[tokio::test]
async fn content_length_completion_drops_producer_in_its_host_runtime() {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };
    struct Producer {
        sent: bool,
        released: Arc<AtomicUsize>,
    }
    impl e::HttpBody for Producer {
        type Data = e::Bytes;
        type Error = std::convert::Infallible;
        fn poll_frame(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<e::Frame<e::Bytes>, Self::Error>>> {
            if self.sent {
                return Poll::Pending;
            }
            self.sent = true;
            Poll::Ready(Some(Ok(e::Frame::data(e::Bytes::from_static(b"body")))))
        }
    }
    impl Drop for Producer {
        fn drop(&mut self) {
            let released = self.released.clone();
            // Mirrors ScT's read-pin cleanup: no EOF poll is required and cleanup
            // starts from Drop on the application's Tokio runtime.
            tokio::runtime::Handle::try_current()
                .expect("producer drop context")
                .spawn(async move {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    released.fetch_add(1, Ordering::SeqCst);
                });
        }
    }
    let runtime = Runtime::new().unwrap();
    runtime
        .scope(async {
            let released = Arc::new(AtomicUsize::new(0));
            let keep = released.clone();
            let router = e::Router::new()
                .unwrap()
                .route(
                    "/",
                    e::MethodRouter::new()
                        .unwrap()
                        .on(e::Method::GET, move |_| {
                            let released = keep.clone();
                            async move {
                                e::http::Response::builder()
                                    .header("content-length", "4")
                                    .body(e::Body::new(Producer {
                                        sent: false,
                                        released,
                                    }))
                                    .unwrap()
                            }
                        })
                        .unwrap(),
                )
                .unwrap();
            let listener = e::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/", listener.local_addr());
            let stop = e::Shutdown::new();
            let server = tokio::spawn(runtime.scope(e::serve(listener, router, stop.clone())));
            let client = simple_server::client::Client::builder()
                .no_proxy()
                .build()
                .unwrap();
            assert_eq!(
                client.get(url).send().await.unwrap().text().await.unwrap(),
                "body"
            );
            tokio::time::timeout(Duration::from_secs(3), async {
                while released.load(Ordering::SeqCst) == 0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap();
            stop.request();
            tokio::time::timeout(Duration::from_secs(3), server)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        })
        .await;
}
