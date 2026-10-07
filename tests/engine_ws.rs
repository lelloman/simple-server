#![cfg(feature = "engine-web")]
use futures_util::{SinkExt, StreamExt};
use simple_server::{
    engine_lifecycle::Shutdown,
    engine_web::{
        self as web, Method, MethodRouter, Router,
        ws::{Message, WebSocketUpgrade},
    },
    runtime::Runtime,
};
use std::{thread, time::Duration};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, client::IntoClientRequest},
};
fn start(
    configure: impl FnOnce() -> Router + Send + 'static,
) -> (String, Shutdown, thread::JoinHandle<()>) {
    let (ready, receiver) = std::sync::mpsc::channel();
    let thread = thread::spawn(move || {
        Runtime::new().unwrap().block_on(async move {
            let router = configure();
            let listener = web::bind("127.0.0.1:0").await.unwrap();
            let shutdown = Shutdown::new();
            ready
                .send((format!("ws://{}/", listener.local_addr()), shutdown.clone()))
                .unwrap();
            web::serve(listener, router, shutdown).await.unwrap();
        })
    });
    let (url, shutdown) = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    (url, shutdown, thread)
}
#[tokio::test]
async fn duplex_echo_protocol_ping_and_close() {
    let (url, stop, thread) = start(|| {
        Router::new()
            .unwrap()
            .route(
                "/",
                MethodRouter::new()
                    .unwrap()
                    .on_handler(Method::GET, |ws: WebSocketUpgrade| async move {
                        ws.protocols(["preferred", "other"])
                            .read_buffer_size(1024)
                            .write_buffer_size(0)
                            .max_write_buffer_size(4096)
                            .on_upgrade(|socket| async move {
                                assert_eq!(socket.protocol().unwrap(), "preferred");
                                let (mut tx, mut rx) = socket.split();
                                while let Some(Ok(message)) = rx.next().await {
                                    match message {
                                        Message::Text(_) | Message::Binary(_) => {
                                            tx.send(message).await.unwrap()
                                        }
                                        Message::Close(_) => {
                                            let _ = tx.close().await;
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            })
                    })
                    .unwrap(),
            )
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut request = url.into_client_request().unwrap();
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "other, preferred".parse().unwrap(),
        );
        let (mut socket, response) = connect_async(request).await.unwrap();
        assert_eq!(response.headers()["sec-websocket-protocol"], "preferred");
        for message in [
            tungstenite::Message::Text("hello λ".into()),
            tungstenite::Message::Binary(vec![0, 255, 42].into()),
        ] {
            socket.send(message.clone()).await.unwrap();
            assert_eq!(socket.next().await.unwrap().unwrap(), message);
        }
        socket
            .send(tungstenite::Message::Ping(vec![1, 2].into()))
            .await
            .unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            tungstenite::Message::Pong(vec![1, 2].into())
        );
        socket.close(None).await.unwrap();
        let _ = socket.next().await;
    })
    .await
    .unwrap();
    stop.request();
    tokio::task::spawn_blocking(move || thread.join().unwrap())
        .await
        .unwrap();
}
#[tokio::test]
async fn rejecting_handshake_and_message_limit() {
    let (url, stop, thread) = start(|| {
        Router::new()
            .unwrap()
            .route(
                "/",
                MethodRouter::new()
                    .unwrap()
                    .on_handler(Method::GET, |ws: WebSocketUpgrade| async move {
                        ws.max_message_size(4).on_upgrade(|mut socket| async move {
                            assert!(socket.recv().await.unwrap().is_err());
                        })
                    })
                    .unwrap(),
            )
            .unwrap()
    });
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let authority = url.trim_start_matches("ws://").trim_end_matches('/');
        let mut tcp = tokio::net::TcpStream::connect(authority).await.unwrap();
        tcp.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        tcp.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        let (mut socket, _) = connect_async(&url).await.unwrap();
        socket
            .send(tungstenite::Message::Text("oversized".into()))
            .await
            .unwrap();
        assert!(!matches!(
            socket.next().await,
            Some(Ok(tungstenite::Message::Text(_)))
        ));
    })
    .await
    .unwrap();
    stop.request();
    tokio::task::spawn_blocking(move || thread.join().unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn cancelled_receive_retains_message_and_pending_reader_does_not_block_writer() {
    let (finished, done) = std::sync::mpsc::channel();
    let (url, stop, thread) = start(move || {
        let finished = finished.clone();
        Router::new()
            .unwrap()
            .route(
                "/",
                MethodRouter::new()
                    .unwrap()
                    .on_handler(Method::GET, move |ws: WebSocketUpgrade| {
                        let finished = finished.clone();
                        async move {
                            ws.on_upgrade(move |mut socket| async move {
                                let mut receive = Box::pin(socket.recv());
                                assert!(futures_util::poll!(receive.as_mut()).is_pending());
                                drop(receive);
                                // The native read is still pending when the writer sends.
                                socket.send(Message::text("ready")).await.unwrap();
                                assert_eq!(
                                    socket.recv().await.unwrap().unwrap(),
                                    Message::text("after cancellation")
                                );
                                socket
                                    .send(Message::Close(Some(web::ws::CloseFrame {
                                        code: web::ws::close_code::RESTART,
                                        reason: "restart λ".into(),
                                    })))
                                    .await
                                    .unwrap();
                                let _ = socket.recv().await;
                                finished.send(()).unwrap();
                            })
                        }
                    })
                    .unwrap(),
            )
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        let (mut socket, _) = connect_async(url).await.unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            tungstenite::Message::Text("ready".into())
        );
        socket
            .send(tungstenite::Message::Text("after cancellation".into()))
            .await
            .unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            tungstenite::Message::Close(Some(tungstenite::protocol::CloseFrame {
                code: tungstenite::protocol::frame::coding::CloseCode::Restart,
                reason: "restart λ".into(),
            }))
        );
        socket.flush().await.unwrap();
    })
    .await
    .unwrap();
    tokio::task::spawn_blocking(move || done.recv_timeout(Duration::from_secs(10)).unwrap())
        .await
        .unwrap();
    stop.request();
    tokio::task::spawn_blocking(move || thread.join().unwrap())
        .await
        .unwrap();
}
