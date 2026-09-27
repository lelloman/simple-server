#![cfg(feature = "sse")]

use futures_util::{FutureExt, Stream, StreamExt, stream};
use http_body_util::BodyExt;
use simple_server::web::{
    IntoResponse, Response, StatusCode,
    sse::{Event, EventError, KeepAlive, Sse},
};
use std::{
    convert::Infallible,
    fmt::Write as _,
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

async fn bytes(response: Response) -> Vec<u8> {
    response
        .into_body()
        .collect(1 << 20)
        .await
        .unwrap()
        .to_vec()
}
fn event_response(event: Event) -> Response {
    Sse::new(stream::iter([Ok::<_, Infallible>(event)])).into_response()
}

#[tokio::test]
async fn event_wire_bytes_and_headers_match_existing_encoder() {
    use simple_server::axum::response::sse::Event as Old;
    let cases = [
        (Event::default(), Old::default()),
        (Event::default().data(""), Old::default().data("")),
        (
            Event::default().data(" 雪\nline\r\n\nend\r"),
            Old::default().data(" 雪\nline\r\n\nend\r"),
        ),
        (Event::default().id(""), Old::default().id("")),
        (
            Event::default().comment("one").comment("two"),
            Old::default().comment("one").comment("two"),
        ),
        (
            Event::default().retry(Duration::from_nanos(999_999)),
            Old::default().retry(Duration::from_nanos(999_999)),
        ),
        (
            Event::default().retry(Duration::MAX),
            Old::default().retry(Duration::MAX),
        ),
        (Event::DEFAULT_KEEP_ALIVE, Old::DEFAULT_KEEP_ALIVE),
        (
            Event::default()
                .event("update")
                .id("42")
                .retry(Duration::from_millis(2001))
                .data("ready"),
            Old::default()
                .event("update")
                .id("42")
                .retry(Duration::from_millis(2001))
                .data("ready"),
        ),
        (
            Event::default()
                .json_data(serde_json::json!({"value":"雪\nline"}))
                .unwrap(),
            Old::default()
                .json_data(serde_json::json!({"value":"雪\nline"}))
                .unwrap(),
        ),
    ];
    for (owned, old) in cases {
        let response = event_response(owned);
        let old_response = simple_server::axum::response::IntoResponse::into_response(
            simple_server::axum::response::Sse::new(stream::iter([Ok::<_, Infallible>(old)])),
        );
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.status(), old_response.status());
        assert_eq!(response.headers(), old_response.headers());
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        assert_eq!(response.headers()["cache-control"], "no-cache");
        let old_bytes = simple_server::axum::body::to_bytes(old_response.into_body(), 1 << 20)
            .await
            .unwrap();
        assert_eq!(bytes(response).await, old_bytes);
    }
    assert_eq!(
        bytes(event_response(
            Event::default()
                .event("update")
                .id("42")
                .retry(Duration::from_millis(2001))
                .data("ready")
        ))
        .await,
        b"event: update\nid: 42\nretry: 2001\ndata: ready\n\n"
    );
}

#[tokio::test]
async fn data_writer_retains_chunked_multiline_and_unicode_formatting() {
    let mut writer = Event::default().id("resume").into_data_writer();
    let snow = "雪";
    writeln!(writer, "snow: {snow}").unwrap();
    writer.write_str("second\r").unwrap();
    writer.write_str("\nthird").unwrap();
    assert_eq!(
        bytes(event_response(writer.into_event())).await,
        "id: resume\ndata: snow: 雪\ndata: second\rdata: \ndata: third\n\n".as_bytes()
    );
    let mut empty = Event::default().into_data_writer();
    empty.write_str("").unwrap();
    assert_eq!(
        bytes(event_response(empty.into_event().data("next"))).await,
        b"data: next\n\n"
    );
}

#[test]
fn invalid_fields_and_duplicate_single_value_fields_are_rejected() {
    let invalid: [fn() -> Event; 9] = [
        || Event::default().event("injected\ndata: secret"),
        || Event::default().comment("injected\rcomment"),
        || Event::default().id("bad\0id"),
        || Event::default().id("bad\nid"),
        || Event::default().event("a").event("b"),
        || Event::default().id("a").id("b"),
        || Event::default().retry(Duration::ZERO).retry(Duration::ZERO),
        || Event::default().data("a").data("b"),
        || Event::default().data("a").json_data(42).unwrap(),
    ];
    for make in invalid {
        assert!(std::panic::catch_unwind(make).is_err());
    }
    assert!(std::panic::catch_unwind(|| KeepAlive::new().text("bad\ncomment")).is_err());
}

#[test]
fn json_serialization_failure_exposes_owned_error_and_source() {
    struct Fail;
    impl serde::Serialize for Fail {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("serialization failed"))
        }
    }
    let error: EventError = Event::default().json_data(Fail).unwrap_err();
    assert!(error.to_string().contains("serialization failed"));
    assert!(std::error::Error::source(&error).is_some());
}

struct ProbeStream {
    polled: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
}
impl Stream for ProbeStream {
    type Item = Result<Event, io::Error>;
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let n = self.polled.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(Some(if n < 2 {
            Ok(Event::default().data(n.to_string()))
        } else {
            Err(io::Error::other("producer failed"))
        }))
    }
}
impl Drop for ProbeStream {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn demand_driven_streaming_preserves_errors_and_drop_releases_producer() {
    let polled = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicBool::new(false));
    let response = Sse::new(ProbeStream {
        polled: polled.clone(),
        dropped: dropped.clone(),
    })
    .into_response();
    assert_eq!(
        polled.load(Ordering::SeqCst),
        0,
        "conversion must not consume the stream"
    );
    let mut body = response.into_body();
    for n in 0..2 {
        assert_eq!(
            body.frame().await.unwrap().unwrap().into_data().unwrap(),
            format!("data: {n}\n\n")
        );
        assert_eq!(polled.load(Ordering::SeqCst), n + 1);
    }
    let error = body.frame().await.unwrap().unwrap_err();
    assert!(error.to_string().contains("producer failed"));
    drop(body);
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn non_unpin_streams_and_response_header_composition_work() {
    let stream = stream::once(async { Ok::<_, Infallible>(Event::default().data("ready")) });
    let response = ([("x-app", "kept")], Sse::new(stream)).into_response();
    assert_eq!(response.headers()["x-app"], "kept");
    assert_eq!(bytes(response).await, b"data: ready\n\n");
}

#[tokio::test(start_paused = true)]
async fn keepalive_is_opt_in_and_default_is_empty_comment_after_fifteen_idle_seconds() {
    let mut plain = Sse::new(stream::pending::<Result<Event, Infallible>>())
        .into_response()
        .into_body();
    let mut alive = Sse::new(stream::pending::<Result<Event, Infallible>>())
        .keep_alive(KeepAlive::default())
        .into_response()
        .into_body();
    assert!(plain.frame().now_or_never().is_none());
    assert!(alive.frame().now_or_never().is_none());
    tokio::time::advance(Duration::from_millis(14_999)).await;
    assert!(alive.frame().now_or_never().is_none());
    tokio::time::advance(Duration::from_millis(1)).await;
    assert_eq!(
        alive.frame().await.unwrap().unwrap().into_data().unwrap(),
        ":\n\n"
    );
    assert!(plain.frame().now_or_never().is_none());
}

#[tokio::test(start_paused = true)]
async fn custom_keepalive_resets_on_events_and_ready_data_wins_over_due_timer() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let stream = stream::poll_fn(move |cx| rx.poll_recv(cx));
    let mut body = Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(5))
                .text("idle"),
        )
        .into_response()
        .into_body();
    assert!(body.frame().now_or_never().is_none());
    tokio::time::advance(Duration::from_secs(5)).await;
    tx.send(Ok::<_, Infallible>(Event::default().data("actual")))
        .await
        .unwrap();
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "data: actual\n\n"
    );
    assert!(body.frame().now_or_never().is_none());
    tokio::time::advance(Duration::from_secs(4)).await;
    assert!(body.frame().now_or_never().is_none());
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        ": idle\n\n"
    );
    drop(body);
    assert!(
        tx.is_closed(),
        "dropping the body must drop the upstream receiver"
    );
}

#[tokio::test(start_paused = true)]
async fn custom_event_keepalive_preserves_event_id_and_json_fields() {
    let heartbeat = Event::default()
        .event("heartbeat")
        .id("10")
        .json_data(serde_json::json!({"alive":true}))
        .unwrap();
    let mut body = Sse::new(stream::pending::<Result<Event, Infallible>>())
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(2))
                .event(heartbeat),
        )
        .into_response()
        .into_body();
    tokio::time::advance(Duration::from_secs(2)).await;
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "event: heartbeat\nid: 10\ndata: {\"alive\":true}\n\n"
    );
}

#[tokio::test(start_paused = true)]
async fn upstream_completion_and_errors_take_priority_over_keepalive() {
    let mut empty = Sse::new(stream::empty::<Result<Event, io::Error>>())
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(1)))
        .into_response()
        .into_body();
    let mut error = Sse::new(stream::iter([Err::<Event, _>(io::Error::other("failed"))]))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(1)))
        .into_response()
        .into_body();
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(empty.frame().await.is_none());
    assert!(error.frame().await.unwrap().is_err());
}

#[cfg(feature = "lifecycle")]
#[tokio::test]
async fn real_http_route_streams_reconnect_ids_and_releases_body_on_disconnect() {
    use simple_server::{
        lifecycle::Shutdown,
        web::{self, HeaderMap, Router, State, routing::get},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct Pending(Arc<AtomicBool>);
    impl Stream for Pending {
        type Item = Result<Event, Infallible>;
        fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }
    impl Drop for Pending {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let app = Router::new()
        .route(
            "/events",
            get(
                |State(dropped): State<Arc<AtomicBool>>, headers: HeaderMap| async move {
                    if headers.get("authorization").map(|h| h.as_bytes()) != Some(b"Bearer test") {
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                    let after: u64 = headers["last-event-id"].to_str().unwrap().parse().unwrap();
                    let event = Event::default()
                        .id((after + 1).to_string())
                        .event("item")
                        .json_data(serde_json::json!({"value":"雪"}))
                        .unwrap();
                    Ok(
                        Sse::new(
                            stream::iter([Ok::<_, Infallible>(event)]).chain(Pending(dropped)),
                        )
                        .keep_alive(KeepAlive::new().interval(Duration::from_millis(20))),
                    )
                },
            ),
        )
        .with_state(dropped.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = Shutdown::new();
    let server = tokio::spawn(web::serve(listener, app, shutdown.clone()));
    let mut rejected = tokio::net::TcpStream::connect(address).await.unwrap();
    rejected
        .write_all(b"GET /events HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), rejected.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with(b"HTTP/1.1 401"));
    assert!(!dropped.load(Ordering::SeqCst));

    let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
    client.write_all(b"GET /events HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test\r\nLast-Event-ID: 41\r\n\r\n").await.unwrap();
    let output = tokio::time::timeout(Duration::from_secs(3), async {
        let mut output = Vec::new();
        loop {
            let mut buffer = [0_u8; 1024];
            let n = client.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            output.extend_from_slice(&buffer[..n]);
            if String::from_utf8_lossy(&output).contains("data: {\"value\":\"雪\"}\n\n") {
                return output;
            }
        }
    })
    .await
    .unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.starts_with("HTTP/1.1 200"));
    assert!(output.contains("content-type: text/event-stream\r\n"));
    assert!(output.contains("cache-control: no-cache\r\n"));
    assert!(output.contains("id: 42\nevent: item\ndata: {\"value\":\"雪\"}\n\n"));
    assert!(
        !dropped.load(Ordering::SeqCst),
        "stream must stay alive after its first event"
    );
    drop(client);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    shutdown.request();
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
