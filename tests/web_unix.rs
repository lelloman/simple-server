#![cfg(all(unix, feature = "unix-http"))]
use simple_server::{
    lifecycle::Shutdown,
    web::{
        self, Body, Request, Response, Router,
        routing::post,
        unix::{UnixClient, UnixHttpErrorKind},
    },
};
use std::time::Duration;

#[tokio::test]
async fn forwards_method_query_headers_and_streamed_body() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("http.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let app = Router::new().route(
        "/echo",
        post(|request: Request| async move {
            assert_eq!(request.uri().query(), Some("a=one%20two&b=3"));
            assert_eq!(request.headers()["host"], "original.example");
            assert_eq!(request.headers().get_all("x-repeat").iter().count(), 2);
            let mut response = Response::new(request.into_body());
            *response.status_mut() = web::http::StatusCode::CREATED;
            response
                .headers_mut()
                .append("x-repeat", "first".parse().unwrap());
            response
                .headers_mut()
                .append("x-repeat", "second".parse().unwrap());
            response
        }),
    );
    let shutdown = Shutdown::new();
    let task = tokio::spawn(web::unix::serve(listener, app, shutdown.clone()));
    let stream = futures_util::stream::iter([
        Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"first")),
        Ok(bytes::Bytes::from_static(b"second")),
    ]);
    let request = Request::builder()
        .method("POST")
        .uri("/echo?a=one%20two&b=3")
        .header("host", "original.example")
        .header("x-repeat", "one")
        .header("x-repeat", "two")
        .body(Body::from_stream(stream))
        .unwrap();
    let response = UnixClient::new().request(&path, request).await.unwrap();
    assert_eq!(response.status(), 201);
    assert_eq!(response.headers().get_all("x-repeat").iter().count(), 2);
    assert_eq!(
        response.into_body().collect(64).await.unwrap(),
        "firstsecond"
    );
    shutdown.request();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(path.exists(), "socket cleanup belongs to the application");
}

#[tokio::test]
async fn invalid_paths_and_transport_failures_are_distinct() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};
    let client = UnixClient::new();
    for path in [
        PathBuf::new(),
        PathBuf::from(OsString::from_vec(vec![0xff])),
    ] {
        let error = client
            .request(path, Request::new(Body::empty()))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), UnixHttpErrorKind::InvalidRequest);
        assert!(std::error::Error::source(&error).is_some());
    }
    let dir = tempfile::tempdir().unwrap();
    let error = client
        .request(dir.path().join("missing.sock"), Request::new(Body::empty()))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), UnixHttpErrorKind::Transport);
    let error = client
        .request(
            dir.path().join("missing.sock"),
            Request::builder().uri("*").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind(), UnixHttpErrorKind::InvalidRequest);
}

#[tokio::test]
async fn shutdown_drains_active_requests() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("drain.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let app = Router::new().route(
        "/",
        post({
            let entered = entered.clone();
            let release = release.clone();
            move || {
                let entered = entered.clone();
                let release = release.clone();
                async move {
                    entered.notify_one();
                    release.notified().await;
                    "completed"
                }
            }
        }),
    );
    let shutdown = Shutdown::new();
    let task = tokio::spawn(web::unix::serve(listener, app, shutdown.clone()));
    let pending = tokio::spawn(async move {
        UnixClient::new()
            .request(
                path,
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .into_body()
            .collect(64)
            .await
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(3), entered.notified())
        .await
        .unwrap();
    shutdown.request();
    assert!(!task.is_finished());
    release.notify_one();
    assert_eq!(pending.await.unwrap(), "completed");
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[cfg(feature = "correlation")]
#[test]
fn uuid_header_matches_rfc_version_and_variant() {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..100 {
        let id = simple_server::correlation::HeaderRequestId::uuid_v4();
        let value = id.as_header_value().to_str().unwrap();
        assert_eq!(value.len(), 36);
        assert_eq!(&value[14..15], "4");
        assert!("89ab".contains(&value[19..20]));
        for (index, byte) in value.bytes().enumerate() {
            if [8, 13, 18, 23].contains(&index) {
                assert_eq!(byte, b'-');
            } else {
                assert!(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            }
        }
        assert!(seen.insert(value.to_owned()));
    }
}

#[tokio::test]
async fn response_stream_is_not_buffered_and_propagates_upstream_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stream.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(1);
    let receiver = std::sync::Arc::new(tokio::sync::Mutex::new(Some(receiver)));
    let app = Router::new().route("/", web::routing::get(move || {
        let receiver = receiver.clone(); async move {
            let receiver = receiver.lock().await.take().unwrap();
            Body::from_stream(futures_util::stream::unfold(receiver, |mut receiver| async move {
                receiver.recv().await.map(|item| (item, receiver))
            }))
        }
    }));
    let shutdown = Shutdown::new();
    let task = tokio::spawn(web::unix::serve(listener, app, shutdown.clone()));
    sender
        .send(Ok(bytes::Bytes::from_static(b"first")))
        .await
        .unwrap();
    let response = tokio::time::timeout(
        Duration::from_secs(3),
        UnixClient::new().request(&path, Request::new(Body::empty())),
    )
    .await
    .unwrap()
    .unwrap();
    let mut stream = response.into_body().into_data_stream();
    use futures_util::StreamExt;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        "first"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(30), stream.next())
            .await
            .is_err()
    );
    sender
        .send(Err(std::io::Error::other("upstream failed")))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(3), stream.next())
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    drop(stream);
    drop(sender);
    shutdown.request();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn pre_requested_shutdown_leaves_socket_ownership_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stopped.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let shutdown = Shutdown::new();
    shutdown.request();
    web::unix::serve(listener, Router::new(), shutdown)
        .await
        .unwrap();
    assert!(path.exists());
    assert!(tokio::net::UnixStream::connect(&path).await.is_err());
}

#[cfg(feature = "ws")]
#[tokio::test]
async fn websocket_upgrade_supports_text_binary_and_close_over_unix() {
    use futures_util::{SinkExt, StreamExt};
    use web::ws::{Message, WebSocketUpgrade};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ws.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let app = Router::new().route(
        "/ws",
        web::routing::get(|ws: WebSocketUpgrade| async {
            ws.on_upgrade(|mut socket| async move {
                while let Some(Ok(message)) = socket.next().await {
                    match message {
                        Message::Text(_) | Message::Binary(_) => {
                            socket.send(message).await.unwrap()
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            })
        }),
    );
    let shutdown = Shutdown::new();
    let task = tokio::spawn(web::unix::serve(listener, app, shutdown.clone()));
    let socket = tokio::net::UnixStream::connect(&path).await.unwrap();
    let (mut client, response) = tokio_tungstenite::client_async("ws://localhost/ws", socket)
        .await
        .unwrap();
    assert_eq!(response.status(), 101);
    for message in [
        tokio_tungstenite::tungstenite::Message::Text("hello".into()),
        tokio_tungstenite::tungstenite::Message::Binary(vec![0, 255, 1].into()),
    ] {
        client.send(message.clone()).await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), client.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            message
        );
    }
    client.close(None).await.unwrap();
    drop(client);
    shutdown.request();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
