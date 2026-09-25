#![cfg(all(
    feature = "multipart-owned",
    feature = "correlation",
    feature = "ws",
    feature = "lifecycle"
))]
use simple_server::web::{self, Body, Request, Router, routing::post};
use tower::ServiceExt;

#[tokio::test]
async fn owned_multipart_preserves_parser_results_and_rejections() {
    async fn shared(mut upload: web::multipart::OwnedMultipart) -> String {
        let mut out = String::new();
        loop {
            match upload.next_field().await {
                Ok(Some(field)) => {
                    out.push_str(field.name().unwrap_or(""));
                    out.push_str(&format!("={:?};", field.text().await));
                }
                Ok(None) => return out,
                Err(error) => return format!("{out}error={error}"),
            }
        }
    }
    async fn legacy(mut upload: axum_extra::extract::Multipart) -> String {
        let mut out = String::new();
        loop {
            match upload.next_field().await {
                Ok(Some(field)) => {
                    out.push_str(field.name().unwrap_or(""));
                    out.push_str(&format!("={:?};", field.text().await));
                }
                Ok(None) => return out,
                Err(error) => return format!("{out}error={error}"),
            }
        }
    }
    let shared = Router::new().route("/", post(shared));
    let legacy = axum::Router::new().route("/", axum::routing::post(legacy));
    for (content_type, body) in [
        ("multipart/form-data; boundary=test", "--test\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n{}\r\n--test\r\nContent-Disposition: form-data; name=\"content\"\r\n\r\nhello\r\n--test--\r\n".to_owned()),
        ("multipart/form-data", "".into()),
        ("application/json", "{}".into()),
        ("multipart/form-data; boundary=test", "--test\r\nbroken".into()),
        ("multipart/form-data; boundary=test", format!("--test\r\nContent-Disposition: form-data; name=\"content\"\r\n\r\n{}\r\n--test--\r\n", "x".repeat(2_100_000))),
    ] {
        let request = || Request::builder().method("POST").uri("/").header("content-type", content_type).body(Body::from(body.clone())).unwrap();
        let old = legacy.clone().oneshot(request()).await.unwrap();
        let new = shared.clone().oneshot(request()).await.unwrap();
        assert_eq!(old.status(), new.status());
        assert_eq!(old.headers(), new.headers());
        assert_eq!(axum::body::to_bytes(old.into_body(), usize::MAX).await.unwrap(), new.into_body().collect(usize::MAX).await.unwrap());
    }
}

#[tokio::test]
async fn owned_fields_enforce_runtime_exclusivity() {
    use web::FromRequest;
    let request = Request::builder()
        .header("content-type", "multipart/form-data; boundary=x")
        .body(Body::from(
            "--x\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\none\r\n--x--\r\n",
        ))
        .unwrap();
    let mut upload = web::multipart::OwnedMultipart::from_request(request, &())
        .await
        .unwrap();
    let field = upload.next_field().await.unwrap().unwrap();
    assert!(upload.next_field().await.is_err());
    assert_eq!(field.text().await.unwrap(), "one");
}

#[tokio::test]
async fn selected_correlation_accepts_standard_http_bodies() {
    use simple_server::correlation::{Correlation, HeaderRequestId, Propagation};
    let id = web::HeaderValue::from_static("opaque id / accepted");
    let response = Correlation::new(web::http::HeaderName::from_static("x-request-id"))
        .run_selected(
            web::http::Request::new(vec![1u8, 2]),
            HeaderRequestId::new(id.clone()),
            Propagation::default(),
            |request| async move {
                assert_eq!(request.body(), &[1, 2]);
                assert!(simple_server::correlation::current_id().is_none());
                assert!(simple_server::correlation::current_header_id().is_some());
                web::http::Response::new("unchanged")
            },
        )
        .await;
    assert_eq!(response.body(), &"unchanged");
    assert_eq!(response.headers()["x-request-id"], id);
    assert!(simple_server::correlation::current_header_id().is_none());
}

#[tokio::test]
async fn websocket_negotiates_server_preference_on_real_http() {
    let app = Router::new().route(
        "/",
        web::routing::get(|ws: web::compat::WebSocketUpgrade| async {
            ws.protocols(["preferred", "alternate"])
                .on_upgrade(|_socket| async {})
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = simple_server::lifecycle::Shutdown::new();
    let task = tokio::spawn(web::serve(listener, app, shutdown.clone()));
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Protocol: alternate, preferred\r\n\r\n").await.unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        socket.read_to_end(&mut response),
    )
    .await
    .unwrap()
    .unwrap();
    let response = String::from_utf8(response).unwrap().to_lowercase();
    assert!(response.starts_with("http/1.1 101"), "{response}");
    assert!(
        response.contains("sec-websocket-protocol: preferred\r\n"),
        "{response}"
    );
    shutdown.request();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn websocket_limits_reject_large_frames_and_fragmented_messages() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let failures = Arc::new(AtomicUsize::new(0));
    let count = failures.clone();
    let app = Router::new().route(
        "/",
        web::routing::get(move |ws: web::compat::WebSocketUpgrade| {
            let count = count.clone();
            async move {
                ws.max_frame_size(4)
                    .max_message_size(6)
                    .on_upgrade(move |mut socket| async move {
                        if let Some(Err(_)) = socket.recv().await {
                            count.fetch_add(1, Ordering::SeqCst);
                        }
                    })
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = simple_server::lifecycle::Shutdown::new();
    let task = tokio::spawn(web::serve(listener, app, shutdown.clone()));
    for frames in [vec![(0x82u8, 5usize)], vec![(0x02, 4), (0x80, 4)]] {
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        socket.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").await.unwrap();
        let mut header = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !header.ends_with(b"\r\n\r\n") {
                header.push(socket.read_u8().await.unwrap());
            }
        })
        .await
        .unwrap();
        assert!(header.starts_with(b"HTTP/1.1 101"));
        let mut wire = Vec::new();
        for (opcode, len) in frames {
            wire.extend_from_slice(&[opcode, 0x80 | len as u8, 1, 2, 3, 4]);
            for i in 0..len {
                wire.push(b'x' ^ [1, 2, 3, 4][i % 4]);
            }
        }
        socket.write_all(&wire).await.unwrap();
        let mut rest = Vec::new();
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            socket.read_to_end(&mut rest),
        )
        .await
        .unwrap();
    }
    shutdown.request();
    task.await.unwrap().unwrap();
    assert_eq!(failures.load(Ordering::SeqCst), 2);
}
