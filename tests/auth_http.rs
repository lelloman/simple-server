#![cfg(all(feature = "auth", feature = "http"))]
use http::{Request, Response, StatusCode, request::Parts};
use simple_server::{
    auth::{AsyncAccess, AuthLayer, Identity},
    axum::{self, Router, body::Body, routing::post},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn real_http_access_failures_never_reach_handler_and_public_routes_stay_public() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let policy = AsyncAccess::new(|parts: &Parts| {
        Box::pin(async move {
            match parts
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
            {
                Some("Bearer admin") => Ok("admin".to_owned()),
                Some("Bearer user") => Ok("user".to_owned()),
                Some("Bearer unavailable") => Err(StatusCode::SERVICE_UNAVAILABLE),
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        })
    })
    .with_check(|user, _| {
        Box::pin(async move {
            if user == "admin" {
                Ok(())
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        })
    });
    let protected = Router::new()
        .route(
            "/protected",
            post(move |req: Request<Body>| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(
                        req.extensions()
                            .get::<Identity<String>>()
                            .unwrap()
                            .principal(),
                        "admin"
                    );
                    let bytes = axum::body::to_bytes(req.into_body(), 1024).await.unwrap();
                    String::from_utf8(bytes.to_vec()).unwrap()
                }
            }),
        )
        .route_layer(AuthLayer::new(policy, |code| {
            Response::builder()
                .status(code)
                .header("www-authenticate", "Bearer")
                .body(Body::from("denied"))
                .unwrap()
        }));
    let app = Router::new()
        .route("/public", post(|| async { "public" }))
        .merge(protected);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    for (path, credential, status, body) in [
        ("/protected", "", 401, "denied"),
        ("/protected", "Bearer wrong", 401, "denied"),
        ("/protected", "Bearer user", 403, "denied"),
        ("/protected", "Bearer unavailable", 503, "denied"),
        ("/protected", "Bearer admin", 200, "payload"),
        ("/public", "Bearer wrong", 200, "public"),
    ] {
        let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        let header = if credential.is_empty() {
            String::new()
        } else {
            format!("Authorization: {credential}\r\n")
        };
        socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\n{header}Content-Length: 7\r\nConnection: close\r\n\r\npayload").as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            socket.read_to_end(&mut bytes),
        )
        .await
        .unwrap()
        .unwrap();
        let response = String::from_utf8(bytes).unwrap();
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{response}"
        );
        assert!(response.ends_with(body), "{response}");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    stop.send(()).unwrap();
    server.await.unwrap();
}
