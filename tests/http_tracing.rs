#![cfg(feature = "http-tracing")]
use futures_util::{FutureExt, StreamExt, stream};
use http_body_util::{BodyExt, StreamBody};
use simple_server::{
    axum::{
        Router,
        body::{Body, Bytes, to_bytes},
        http::{HeaderMap, Request, Response, StatusCode},
        middleware::{self, Next},
        routing::get,
    },
    http_tracing::trace,
};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Capture {
    fn install(&self) -> tracing::subscriber::DefaultGuard {
        let sink = self.clone();
        tracing::subscriber::set_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .without_time()
                .with_ansi(false)
                .json()
                .with_writer(move || sink.clone())
                .finish(),
        )
    }
    fn events(&self) -> Vec<serde_json::Value> {
        String::from_utf8(self.0.lock().unwrap().clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
    fn finished(&self) -> Vec<serde_json::Value> {
        self.events()
            .into_iter()
            .filter(|v| v["fields"]["message"] == "http.finished")
            .collect()
    }
}
async fn middleware(request: Request<Body>, next: Next) -> Response<Body> {
    trace(request, |request| next.run(request)).await
}
fn request() -> Request<Body> {
    Request::new(Body::empty())
}

#[tokio::test]
async fn production_router_templates_status_and_no_sensitive_values() {
    let capture = Capture::default();
    let _guard = capture.install();
    let app = Router::new()
        .route(
            "/users/{id}",
            get(|| async { (StatusCode::CREATED, "private-body") }),
        )
        .layer(middleware::from_fn(middleware));
    for (uri, method, status) in [
        ("/users/private-path?secret-query=1", "GET", 201),
        ("/users/private-path", "POST", 405),
        ("/private-fallback", "GET", 404),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method(method)
                    .header("authorization", "private-credential")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        to_bytes(response.into_body(), usize::MAX).await.unwrap();
    }
    let logs = capture.text();
    for secret in [
        "private-path",
        "secret-query",
        "private-body",
        "private-fallback",
        "private-credential",
    ] {
        assert!(!logs.contains(secret), "{logs}");
    }
    assert!(logs.contains("/users/{id}"));
    assert!(logs.contains("<unmatched>"));
    let finished = capture.finished();
    assert_eq!(finished.len(), 3);
    for (event, status) in finished.iter().zip([201, 405, 404]) {
        assert_eq!(event["span"]["status"], status);
        assert_eq!(event["fields"]["outcome"], "complete");
    }
}

#[tokio::test]
async fn streaming_preserves_data_trailers_and_separates_timing() {
    let capture = Capture::default();
    let _guard = capture.install();
    let mut trailers = HeaderMap::new();
    trailers.insert("x-trailer", "kept".parse().unwrap());
    let frames = vec![
        Ok::<_, io::Error>(http_body::Frame::data(Bytes::from_static(b"payload"))),
        Ok(http_body::Frame::trailers(trailers.clone())),
    ];
    let response = trace(request(), |_| async {
        Response::new(Body::new(StreamBody::new(stream::iter(frames))))
    })
    .await;
    assert!(capture.finished().is_empty());
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    let mut body = response.into_body();
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "payload"
    );
    assert_eq!(
        body.frame()
            .await
            .unwrap()
            .unwrap()
            .into_trailers()
            .unwrap(),
        trailers
    );
    assert!(body.frame().await.is_none());
    drop(body);
    let finished = capture.finished();
    assert_eq!(finished.len(), 1);
    let events = capture.events();
    let headers = events
        .iter()
        .find(|v| v["fields"]["message"] == "http.response_headers")
        .unwrap();
    assert!(
        finished[0]["fields"]["duration_ms"].as_f64().unwrap()
            >= headers["fields"]["header_latency_ms"].as_f64().unwrap() + 10.0
    );
}

#[tokio::test]
async fn body_errors_are_not_logged_as_completion_or_leaked() {
    let capture = Capture::default();
    let _guard = capture.install();
    let body = Body::from_stream(stream::iter([Err::<Bytes, _>(io::Error::other(
        "private-error-detail",
    ))]));
    let response = trace(request(), |_| async { Response::new(body) }).await;
    assert!(to_bytes(response.into_body(), usize::MAX).await.is_err());
    let events = capture.finished();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["fields"]["outcome"], "error");
    assert!(!capture.text().contains("private-error-detail"));
}

#[tokio::test]
async fn dropped_future_and_unconsumed_body_have_distinct_cancellation_phases() {
    let capture = Capture::default();
    let _guard = capture.install();
    assert!(
        trace(request(), |_| std::future::pending())
            .now_or_never()
            .is_none()
    );
    let body = Body::from_stream(stream::pending::<Result<Bytes, io::Error>>());
    drop(trace(request(), |_| async { Response::new(body) }).await);
    let events = capture.finished();
    assert_eq!(events.len(), 2);
    for event in &events {
        assert_eq!(event["fields"]["outcome"], "cancelled");
    }
    assert_eq!(events[0]["fields"]["phase"], "headers");
    assert_eq!(events[1]["fields"]["phase"], "body");
}

#[tokio::test]
async fn upgrades_end_at_handoff_and_empty_responses_complete_once() {
    let capture = Capture::default();
    let _guard = capture.install();
    for (method, status) in [
        ("GET", 101),
        ("CONNECT", 200),
        ("CONNECT", 403),
        ("HEAD", 200),
    ] {
        let request = Request::builder()
            .method(method)
            .body(Body::empty())
            .unwrap();
        let response = trace(request, |_| async {
            Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap()
        })
        .await;
        assert_eq!(response.status().as_u16(), status);
        drop(response);
    }
    let events = capture.finished();
    assert_eq!(events.len(), 4);
    for (event, outcome) in events
        .iter()
        .zip(["upgraded", "upgraded", "complete", "complete"])
    {
        assert_eq!(event["fields"]["outcome"], outcome);
    }
}

#[tokio::test]
async fn callback_construction_body_polling_and_custom_methods_are_scoped() {
    let capture = Capture::default();
    let _guard = capture.install();
    let request = Request::builder()
        .method("PRIVATE-METHOD")
        .body(Body::empty())
        .unwrap();
    let response = trace(request, |_| {
        tracing::info!("callback-created");
        async {
            Response::new(Body::from_stream(stream::once(async {
                tracing::info!("body-polled");
                Ok::<_, io::Error>(Bytes::from_static(b"ok"))
            })))
        }
    })
    .await;
    to_bytes(response.into_body(), usize::MAX).await.unwrap();
    for event in capture.events().iter().filter(|v| {
        ["callback-created", "body-polled"].contains(&v["fields"]["message"].as_str().unwrap_or(""))
    }) {
        assert_eq!(event["span"]["name"], "http.request");
        assert_eq!(event["span"]["method"], "OTHER");
    }
    assert!(!capture.text().contains("PRIVATE-METHOD"));
}

#[cfg(feature = "correlation")]
#[tokio::test]
async fn validated_correlation_is_captured_and_retained_during_body_polling() {
    use simple_server::correlation::{Correlation, IncomingIds};
    let capture = Capture::default();
    let _guard = capture.install();
    let correlation = Correlation::new("x-request-id".parse().unwrap())
        .incoming_ids(IncomingIds::AcceptValidated);
    let run = |id: &'static str| {
        let correlation = correlation.clone();
        async move {
            let request = Request::builder()
                .header("x-request-id", id)
                .body(Body::empty())
                .unwrap();
            correlation
                .run(request, |request| {
                    trace(request, |_| async {
                        tokio::task::yield_now().await;
                        Response::new(Body::from("ok"))
                    })
                })
                .await
        }
    };
    let (a, b) = tokio::join!(run("one"), run("two"));
    to_bytes(a.into_body(), usize::MAX).await.unwrap();
    to_bytes(b.into_body(), usize::MAX).await.unwrap();
    let events = capture.finished();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["span"]["request_id"], "one");
    assert_eq!(events[1]["span"]["request_id"], "two");
}

#[tokio::test]
async fn final_frame_size_hint_and_partial_drop_are_preserved() {
    let capture = Capture::default();
    let _guard = capture.install();
    let response = trace(request(), |_| async { Response::new(Body::from("abc")) }).await;
    let mut body = response.into_body();
    assert_eq!(http_body::Body::size_hint(&body).exact(), Some(3));
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "abc"
    );
    assert!(http_body::Body::is_end_stream(&body));
    drop(body); // EOF need not be polled when the final frame reports end-of-stream.
    let frames =
        stream::iter([Ok::<_, io::Error>(Bytes::from_static(b"first"))]).chain(stream::pending());
    let response = trace(request(), |_| async {
        Response::new(Body::from_stream(frames))
    })
    .await;
    let mut body = response.into_body();
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "first"
    );
    drop(body);
    let events = capture.finished();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["fields"]["outcome"], "complete");
    assert_eq!(events[1]["fields"]["outcome"], "cancelled");
}

#[cfg(feature = "correlation")]
#[tokio::test]
async fn opaque_correlation_is_not_automatically_logged() {
    use simple_server::correlation::{Correlation, HeaderRequestId, Propagation};
    let capture = Capture::default();
    let _guard = capture.install();
    Correlation::new("x-id".parse().unwrap())
        .run_selected(
            request(),
            HeaderRequestId::new("private legacy value".parse().unwrap()),
            Propagation::default(),
            |request| trace(request, |_| async { Response::new(Body::empty()) }),
        )
        .await;
    assert!(!capture.text().contains("private legacy value"));
    assert_eq!(capture.finished().len(), 1);
}

#[tokio::test]
async fn server_error_is_visible_before_a_stalled_body_finishes() {
    let capture = Capture::default();
    let _guard = capture.install();
    let response = trace(request(), |_| async {
        Response::builder()
            .status(503)
            .body(Body::from_stream(
                stream::pending::<Result<Bytes, io::Error>>(),
            ))
            .unwrap()
    })
    .await;
    let events = capture.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["level"], "ERROR");
    assert_eq!(events[0]["fields"]["message"], "http.response_headers");
    assert_eq!(events[0]["fields"]["status"], 503);
    assert!(capture.finished().is_empty());
    drop(response);
    assert_eq!(capture.finished()[0]["fields"]["outcome"], "cancelled");
}
