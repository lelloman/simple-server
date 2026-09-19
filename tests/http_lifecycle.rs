#![cfg(all(feature = "lifecycle", feature = "http"))]

use std::{future::pending, io, net::SocketAddr, time::Duration};

use simple_server::{
    axum::{Router, body::Body, extract::ConnectInfo, routing::get},
    http,
    lifecycle::{Lifecycle, LifecycleError, Shutdown, ShutdownOptions, ShutdownReason, Unfinished},
};
#[cfg(feature = "ws")]
use tokio::sync::oneshot;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

async fn request(address: SocketAddr) -> io::Result<(TcpStream, SocketAddr)> {
    let mut stream = TcpStream::connect(address).await?;
    let peer = stream.local_addr()?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await?;
    Ok((stream, peer))
}

async fn wait_closed(address: SocketAddr) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if TcpStream::connect(address).await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("listener did not close");
}

#[tokio::test]
async fn two_listeners_drain_active_request_and_preserve_peer_address() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut lifecycle = Lifecycle::new(ShutdownOptions {
            grace_period: Duration::from_secs(3),
        });
        let shutdown = lifecycle.shutdown();
        let entered = Shutdown::new();
        let release = Shutdown::new();
        let app = Router::new().route(
            "/",
            get({
                let entered = entered.clone();
                let release = release.clone();
                move |ConnectInfo(peer): ConnectInfo<SocketAddr>| async move {
                    entered.request();
                    release.requested().await;
                    format!("drained-{peer}")
                }
            }),
        );
        let api = http::bind("127.0.0.1:0").await.unwrap();
        let api_address = api.local_addr().unwrap();
        let metrics = http::bind("127.0.0.1:0").await.unwrap();
        let metrics_address = metrics.local_addr().unwrap();
        lifecycle
            .service(
                "api",
                http::serve(
                    api,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                    shutdown.clone(),
                ),
            )
            .unwrap();
        lifecycle
            .service(
                "metrics",
                http::serve(metrics, Router::new(), shutdown.clone()),
            )
            .unwrap();
        let server = tokio::spawn(
            lifecycle.run(pending::<io::Result<ShutdownReason>>(), async {
                Ok::<_, io::Error>(())
            }),
        );
        let (mut stream, peer) = request(api_address).await.unwrap();
        entered.requested().await;
        shutdown.request();
        wait_closed(api_address).await;
        wait_closed(metrics_address).await;
        assert!(!server.is_finished(), "active request must keep drain open");
        release.request();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains(&format!("drained-{peer}")), "{response}");
        server.await.unwrap().unwrap();
    })
    .await
    .expect("HTTP drain test timed out");
}

#[tokio::test]
async fn bind_conflict_preserves_io_error_and_pre_requested_shutdown_drops_listener() {
    let listener = http::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let error = http::bind(address).await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
    let shutdown = Shutdown::new();
    shutdown.request();
    http::serve(listener, Router::new(), shutdown)
        .await
        .unwrap();
    assert!(TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn unfinished_stream_hits_deadline_instead_of_reporting_success() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let app = Router::new().route(
            "/",
            get(|| async {
                Body::from_stream(
                    futures_util::stream::once(async { Ok::<_, io::Error>("first chunk\n") })
                        .chain(futures_util::stream::pending::<
                            Result<&'static str, io::Error>,
                        >()),
                )
            }),
        );
        let listener = http::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut lifecycle = Lifecycle::new(ShutdownOptions {
            grace_period: Duration::from_millis(50),
        });
        let shutdown = lifecycle.shutdown();
        lifecycle
            .service("stream", http::serve(listener, app, shutdown.clone()))
            .unwrap();
        let server = tokio::spawn(
            lifecycle.run(pending::<io::Result<ShutdownReason>>(), async {
                Ok::<_, io::Error>(())
            }),
        );
        let (mut stream, _) = request(address).await.unwrap();
        let mut bytes = [0; 1024];
        assert!(stream.read(&mut bytes).await.unwrap() > 0);
        shutdown.request();
        let LifecycleError::Shutdown(report) = server.await.unwrap().unwrap_err() else {
            panic!("expected timeout")
        };
        assert_eq!(
            report.unfinished,
            Some(Unfinished::Services(vec!["stream".into()]))
        );
        // The client and test runtime own final connection teardown. The timeout
        // deliberately makes no claim that dropping serve kills connection tasks.
    })
    .await
    .expect("stream test timed out");
}

use futures_util::StreamExt;

#[cfg(feature = "ws")]
#[tokio::test]
async fn upgraded_connection_needs_explicit_cancellation_and_tracking() {
    use simple_server::axum::extract::WebSocketUpgrade;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut lifecycle = Lifecycle::new(ShutdownOptions { grace_period: Duration::from_secs(3) });
        let shutdown = lifecycle.shutdown();
        let release = Shutdown::new();
        let entered = Shutdown::new();
        let (done_tx, done_rx) = oneshot::channel();
        let done_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(done_tx)));
        let app = Router::new().route("/", get({
            let shutdown = shutdown.clone();
            let release = release.clone();
            let entered = entered.clone();
            move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    entered.request();
                    shutdown.requested().await;
                    release.requested().await;
                    drop(socket);
                    if let Some(done) = done_tx.lock().unwrap().take() { let _ = done.send(()); }
                })
            }
        }));
        let listener = http::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        lifecycle.service("http", http::serve(listener, app, shutdown.clone())).unwrap();
        lifecycle.service("upgraded-connections", async {
            done_rx.await.map_err(io::Error::other)
        }).unwrap();
        let server = tokio::spawn(lifecycle.run(pending::<io::Result<ShutdownReason>>(), async { Ok::<_, io::Error>(()) }));
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").await.unwrap();
        let mut bytes = [0; 1024];
        let read = stream.read(&mut bytes).await.unwrap();
        assert!(String::from_utf8_lossy(&bytes[..read]).starts_with("HTTP/1.1 101"));
        entered.requested().await;
        shutdown.request();
        wait_closed(address).await;
        assert!(!server.is_finished());
        release.request();
        server.await.unwrap().unwrap();
    }).await.expect("WebSocket test timed out");
}
