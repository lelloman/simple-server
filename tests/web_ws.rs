#![cfg(all(feature = "web", feature = "ws", feature = "lifecycle"))]
use futures_util::{SinkExt, StreamExt};
use simple_server::{
    lifecycle::Shutdown,
    web::{
        self, Router,
        ws::{CloseFrame, Message, WebSocketUpgrade, close_code},
    },
};
use std::time::Duration;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, client::IntoClientRequest},
};

async fn server(
    app: Router,
) -> (
    String,
    Shutdown,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/", listener.local_addr().unwrap());
    let shutdown = Shutdown::new();
    let task = tokio::spawn(web::serve(listener, app, shutdown.clone()));
    (url, shutdown, task)
}
async fn stop(shutdown: Shutdown, task: tokio::task::JoinHandle<std::io::Result<()>>) {
    shutdown.request();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn split_echo_preserves_text_binary_and_negotiation() {
    let app = Router::new().route(
        "/",
        web::routing::get(|ws: WebSocketUpgrade| async {
            let ws = ws.protocols(["preferred", "alternate"]);
            assert_eq!(ws.selected_protocol().unwrap(), "preferred");
            assert!(ws.requested_protocols().next().is_some());
            ws.read_buffer_size(1024)
                .write_buffer_size(0)
                .max_write_buffer_size(4096)
                .on_failed_upgrade(|error| panic!("{error}"))
                .on_upgrade(|socket| async {
                    assert_eq!(socket.protocol().unwrap(), "preferred");
                    let (mut sender, mut receiver) = socket.split();
                    while let Some(Ok(message)) = receiver.next().await {
                        match message {
                            Message::Text(_) | Message::Binary(_) => {
                                sender.send(message).await.unwrap()
                            }
                            Message::Close(_) => {
                                let _ = sender.close().await;
                                break;
                            }
                            _ => {}
                        }
                    }
                })
        }),
    );
    let (url, shutdown, task) = server(app).await;
    let mut request = url.into_client_request().unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "alternate, preferred".parse().unwrap(),
    );
    let (mut client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.headers()["sec-websocket-protocol"], "preferred");
    for message in [
        tungstenite::Message::Text("héllo".into()),
        tungstenite::Message::Binary(vec![0, 255, 1].into()),
    ] {
        client.send(message.clone()).await.unwrap();
        assert_eq!(client.next().await.unwrap().unwrap(), message);
    }
    client.close(None).await.unwrap();
    assert!(matches!(
        client.next().await.unwrap().unwrap(),
        tungstenite::Message::Close(_)
    ));
    drop(client);
    stop(shutdown, task).await;
}
#[tokio::test]
async fn automatic_pong_and_client_close_reason_reach_owned_socket() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    let app = Router::new().route(
        "/",
        web::routing::get(move |ws: WebSocketUpgrade| {
            let tx = tx.clone();
            async move {
                ws.on_upgrade(move |mut socket| async move {
                    while let Some(Ok(message)) = socket.recv().await {
                        let closed = matches!(message, Message::Close(_));
                        tx.send(message).await.unwrap();
                        if closed {
                            let _ = socket.close().await;
                            break;
                        }
                    }
                })
            }
        }),
    );
    let (url, shutdown, task) = server(app).await;
    let (mut client, _) = connect_async(url).await.unwrap();
    client
        .send(tungstenite::Message::Ping(b"beat".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        rx.recv().await.unwrap(),
        Message::Ping(b"beat".to_vec().into())
    );
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), client.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        tungstenite::Message::Pong(b"beat".to_vec().into())
    );
    client
        .send(tungstenite::Message::Pong(b"unsolicited".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        rx.recv().await.unwrap(),
        Message::Pong(b"unsolicited".to_vec().into())
    );
    client
        .close(Some(tungstenite::protocol::CloseFrame {
            code: tungstenite::protocol::frame::coding::CloseCode::Away,
            reason: "bye ✓".into(),
        }))
        .await
        .unwrap();
    assert_eq!(
        rx.recv().await.unwrap(),
        Message::Close(Some(CloseFrame {
            code: close_code::AWAY,
            reason: "bye ✓".into()
        }))
    );
    let _ = client.next().await;
    drop(client);
    stop(shutdown, task).await;
}
#[tokio::test]
async fn server_close_preserves_application_code_and_reason() {
    let app = Router::new().route(
        "/",
        web::routing::get(|ws: WebSocketUpgrade| async {
            ws.on_upgrade(|mut socket| async move {
                socket
                    .send(Message::Close(Some(CloseFrame {
                        code: 4001,
                        reason: "session expired".into(),
                    })))
                    .await
                    .unwrap();
                let _ = socket.recv().await;
            })
        }),
    );
    let (url, shutdown, task) = server(app).await;
    let (mut client, _) = connect_async(url).await.unwrap();
    let tungstenite::Message::Close(Some(frame)) = client.next().await.unwrap().unwrap() else {
        panic!("missing close");
    };
    assert_eq!(u16::from(frame.code), 4001);
    assert_eq!(frame.reason.as_str(), "session expired");
    let _ = client.flush().await;
    drop(client);
    stop(shutdown, task).await;
}
#[tokio::test]
async fn no_common_protocol_is_optional_and_manual_selection_works() {
    for manual in [false, true] {
        let app = Router::new().route(
            "/",
            web::routing::get(move |ws: WebSocketUpgrade| async move {
                let mut ws = ws.protocols(["absent"]);
                assert!(ws.selected_protocol().is_none());
                if manual {
                    ws.set_selected_protocol(web::HeaderValue::from_static("offered"));
                }
                ws.on_upgrade(|mut socket| async move {
                    socket.send(Message::text("ready")).await.unwrap();
                })
            }),
        );
        let (url, shutdown, task) = server(app).await;
        let mut request = url.into_client_request().unwrap();
        request
            .headers_mut()
            .insert("sec-websocket-protocol", "offered".parse().unwrap());
        if !manual {
            // This client requires a selected protocol whenever it offers one.
            // Its precise error confirms the server accepted without selecting.
            let error = connect_async(request).await.unwrap_err();
            assert!(matches!(
                error,
                tungstenite::Error::Protocol(
                    tungstenite::error::ProtocolError::SecWebSocketSubProtocolError(
                        tungstenite::error::SubProtocolError::NoSubProtocol
                    )
                )
            ));
            stop(shutdown, task).await;
            continue;
        }
        let (mut client, response) = connect_async(request).await.unwrap();
        assert_eq!(
            response.headers().get("sec-websocket-protocol").is_some(),
            manual
        );
        assert_eq!(
            client.next().await.unwrap().unwrap(),
            tungstenite::Message::Text("ready".into())
        );
        drop(client);
        stop(shutdown, task).await;
    }
}
#[tokio::test]
async fn upgrade_rejections_match_backend_status_headers_and_body() {
    use tower::ServiceExt;
    let shared = Router::new().route("/", web::routing::any(|_: WebSocketUpgrade| async {}));
    let legacy = axum::Router::new().route(
        "/",
        axum::routing::any(|_: axum::extract::WebSocketUpgrade| async {}),
    );
    for (method, version, connection, upgrade, key, ws_version) in [
        (
            "POST",
            http::Version::HTTP_11,
            "upgrade",
            "websocket",
            Some("abc"),
            "13",
        ),
        (
            "GET",
            http::Version::HTTP_11,
            "close",
            "websocket",
            Some("abc"),
            "13",
        ),
        (
            "GET",
            http::Version::HTTP_11,
            "upgrade",
            "other",
            Some("abc"),
            "13",
        ),
        (
            "GET",
            http::Version::HTTP_11,
            "upgrade",
            "websocket",
            None,
            "13",
        ),
        (
            "GET",
            http::Version::HTTP_11,
            "upgrade",
            "websocket",
            Some("abc"),
            "12",
        ),
        (
            "GET",
            http::Version::HTTP_10,
            "upgrade",
            "websocket",
            Some("abc"),
            "13",
        ),
        (
            "GET",
            http::Version::HTTP_2,
            "upgrade",
            "websocket",
            Some("abc"),
            "13",
        ),
        (
            "CONNECT",
            http::Version::HTTP_2,
            "upgrade",
            "websocket",
            Some("abc"),
            "13",
        ),
    ] {
        let request = || {
            let mut r = web::Request::builder()
                .method(method)
                .version(version)
                .uri("/")
                .header("connection", connection)
                .header("upgrade", upgrade)
                .header("sec-websocket-version", ws_version);
            if let Some(key) = key {
                r = r.header("sec-websocket-key", key);
            }
            r.body(web::Body::empty()).unwrap()
        };
        let a = shared.clone().oneshot(request()).await.unwrap();
        let b = legacy.clone().oneshot(request()).await.unwrap();
        assert_eq!(a.status(), b.status());
        assert_eq!(a.headers(), b.headers());
        assert_eq!(
            a.into_body().collect(usize::MAX).await.unwrap(),
            axum::body::to_bytes(b.into_body(), usize::MAX)
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
async fn raw_frames_preserve_fragmentation_limits_utf8_and_masking_rules() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for (accept_unmasked, frames, expected) in [
        (
            false,
            vec![(0x01, vec![b'h', 0xc3], true), (0x80, vec![0xa9], true)],
            Some(Message::text("hé")),
        ),
        (false, vec![(0x82, vec![0; 5], true)], None),
        (
            false,
            vec![(0x02, vec![0; 4], true), (0x80, vec![0; 4], true)],
            None,
        ),
        (false, vec![(0x81, vec![0xff], true)], None),
        (false, vec![(0x81, vec![b'x'], false)], None),
        (
            true,
            vec![(0x81, vec![b'x'], false)],
            Some(Message::text("x")),
        ),
    ] {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let app = Router::new().route(
            "/",
            web::routing::get(move |ws: WebSocketUpgrade| {
                let tx = tx.clone();
                async move {
                    ws.max_frame_size(4)
                        .max_message_size(6)
                        .accept_unmasked_frames(accept_unmasked)
                        .on_upgrade(move |mut socket| async move {
                            tx.send(socket.recv().await.unwrap()).await.unwrap();
                        })
                }
            }),
        );
        let (url, shutdown, task) = server(app).await;
        let addr = url.trim_start_matches("ws://").trim_end_matches('/');
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(client.read_u8().await.unwrap());
        }
        assert!(headers.starts_with(b"HTTP/1.1 101"));
        for (opcode, bytes, masked) in frames {
            let mut wire = vec![opcode, bytes.len() as u8 | if masked { 128 } else { 0 }];
            if masked {
                wire.extend([1, 2, 3, 4]);
            }
            wire.extend(
                bytes
                    .iter()
                    .enumerate()
                    .map(|(i, b)| if masked { b ^ [1, 2, 3, 4][i % 4] } else { *b }),
            );
            client.write_all(&wire).await.unwrap();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        match expected {
            Some(message) => assert_eq!(result.unwrap(), message),
            None => {
                let error = result.unwrap_err();
                assert!(!error.to_string().is_empty());
                assert!(std::error::Error::source(&error).is_some());
            }
        }
        drop(client);
        stop(shutdown, task).await;
    }
}
#[test]
fn text_payload_validation_and_message_access_do_not_require_backend_types() {
    use web::ws::Utf8Bytes;
    assert!(Utf8Bytes::try_from(vec![255]).is_err());
    let text = Utf8Bytes::try_from(web::Bytes::from_static(b"hello")).unwrap();
    assert_eq!(Message::Text(text).into_data(), b"hello"[..]);
    assert!(Message::binary(vec![255]).into_text().is_err());
    assert_eq!(Message::Close(None).to_text().unwrap(), "");
    assert_eq!(
        Message::Close(Some(CloseFrame {
            code: 4000,
            reason: "bye".into()
        }))
        .into_text()
        .unwrap()
        .as_str(),
        "bye"
    );
}

#[tokio::test]
async fn background_upgrade_failure_uses_owned_error_callback() {
    use tower::ServiceExt;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let app = Router::new().route(
        "/",
        web::routing::get(move |ws: WebSocketUpgrade| {
            let tx = tx.clone();
            async move {
                ws.on_failed_upgrade(move |error| {
                    tx.try_send(error).unwrap();
                })
                .on_upgrade(|_| async {
                    panic!("failed transport cannot run socket callback");
                })
            }
        }),
    );
    let mut request = web::Request::builder()
        .uri("/")
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(web::Body::empty())
        .unwrap();
    // No Hyper server owns this request, so its upgrade future fails deterministically.
    let upgrade = hyper::upgrade::on(&mut request);
    request.extensions_mut().insert(upgrade);
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        web::StatusCode::SWITCHING_PROTOCOLS
    );
    let error = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(std::error::Error::source(&error).is_some());
    assert!(!error.to_string().is_empty());
}
