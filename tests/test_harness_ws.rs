#![cfg(feature = "test-harness-ws")]
use simple_server::{
    testing::{TestOptions, TestServer, WebSocketOutcome},
    web::{
        self, IntoResponse, Router,
        ws::{CloseFrame, Message, WebSocketUpgrade},
    },
};
use std::time::Duration;
fn app() -> Router {
    Router::new()
        .route(
            "/ws",
            web::routing::get(|ws: WebSocketUpgrade| async {
                ws.protocols(["test"]).on_upgrade(|mut socket| async move {
                    while let Some(Ok(message)) = socket.recv().await {
                        match message {
                            Message::Text(_) | Message::Binary(_) => {
                                socket.send(message).await.unwrap()
                            }
                            Message::Close(_) => {
                                socket.close().await.unwrap();
                                break;
                            }
                            _ => {}
                        }
                    }
                })
            }),
        )
        .route(
            "/auth",
            web::routing::get(|headers: web::HeaderMap, ws: WebSocketUpgrade| async move {
                if headers
                    .get("authorization")
                    .is_none_or(|h| h != "Bearer valid")
                {
                    return web::StatusCode::UNAUTHORIZED.into_response();
                }
                ws.on_upgrade(|mut socket| async move {
                    socket.send(Message::text("authenticated")).await.unwrap();
                    let _ = socket.recv().await;
                })
                .into_response()
            }),
        )
        .route(
            "/reject",
            web::routing::get(|| async { (web::StatusCode::UNAUTHORIZED, "no") }),
        )
}
#[tokio::test]
async fn frames_and_protocol_headers() {
    let server = TestServer::tcp(app()).await.unwrap();
    let WebSocketOutcome::Connected(mut socket) = server
        .get("/ws")
        .header("sec-websocket-protocol", "test")
        .websocket()
        .await
        .unwrap()
    else {
        panic!("rejected");
    };
    assert_eq!(socket.headers()["sec-websocket-protocol"], "test");
    for message in [Message::text("héllo"), Message::binary(vec![0, 255, 1])] {
        socket.send(message.clone()).await.unwrap();
        assert_eq!(socket.receive().await.unwrap(), Some(message));
    }
    socket.send(Message::Ping(vec![1, 2].into())).await.unwrap();
    assert_eq!(
        socket.receive().await.unwrap(),
        Some(Message::Pong(vec![1, 2].into()))
    );
    socket
        .send(Message::Close(Some(CloseFrame {
            code: 1000,
            reason: "done".into(),
        })))
        .await
        .unwrap();
    assert!(matches!(
        socket.receive().await.unwrap(),
        Some(Message::Close(_))
    ));
    drop(socket);
    server.shutdown().await.unwrap();
}
#[tokio::test]
async fn rejection_and_in_process_error() {
    let server = TestServer::tcp(app()).await.unwrap();
    let WebSocketOutcome::Rejected(response) = server
        .get("/reject")
        .header("authorization", "Bearer invalid")
        .websocket()
        .await
        .unwrap()
    else {
        panic!("connected");
    };
    response.assert_status(web::StatusCode::UNAUTHORIZED);
    server.shutdown().await.unwrap();
    assert!(TestServer::new(app()).get("/ws").websocket().await.is_err());
}
#[tokio::test]
async fn idle_receive_times_out_and_drop_cleans_up() {
    let server = TestServer::tcp(app())
        .await
        .unwrap()
        .with_options(TestOptions {
            timeout: Duration::from_millis(100),
            ..Default::default()
        });
    let WebSocketOutcome::Connected(mut socket) = server.get("/ws").websocket().await.unwrap()
    else {
        panic!();
    };
    assert!(socket.receive().await.is_err());
    socket.close().await.unwrap();
    drop(socket);
    server.shutdown().await.unwrap();
}
#[tokio::test]
async fn incoming_message_limit() {
    let server = TestServer::tcp(app())
        .await
        .unwrap()
        .with_options(TestOptions {
            max_response_bytes: 2,
            ..Default::default()
        });
    let WebSocketOutcome::Connected(mut socket) = server.get("/ws").websocket().await.unwrap()
    else {
        panic!();
    };
    socket.send(Message::text("too large")).await.unwrap();
    assert!(socket.receive().await.is_err());
    drop(socket);
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn authenticated_handshake() {
    let server = TestServer::tcp(app()).await.unwrap();
    let WebSocketOutcome::Rejected(response) = server.get("/auth").websocket().await.unwrap()
    else {
        panic!();
    };
    response.assert_status(web::StatusCode::UNAUTHORIZED);
    let WebSocketOutcome::Connected(mut socket) = server
        .get("/auth")
        .header("authorization", "Bearer valid")
        .websocket()
        .await
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(
        socket.receive().await.unwrap(),
        Some(Message::text("authenticated"))
    );
    socket.close().await.unwrap();
    drop(socket);
    server.shutdown().await.unwrap();
}
