#![cfg(all(unix, feature = "engine-web"))]
use simple_server::{
    engine_web::{self as e, Body, Method, MethodRouter, Response, Router, Shutdown, unix},
    runtime::{JoinHandle, spawn, spawn_blocking},
    time::{sleep, timeout},
};
use std::{
    io::{self, Read, Write},
    os::unix::{ffi::OsStringExt, net::UnixStream},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
fn connect(path: &Path) -> UnixStream {
    let socket = UnixStream::connect(path).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
}
async fn finish(stop: Shutdown, server: JoinHandle<io::Result<()>>) {
    stop.request();
    timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
#[simple_server::test]
async fn binding_preserves_paths_existing_files_errors_and_drop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join(std::ffi::OsString::from_vec(b"socket-\xff".to_vec()));
    let listener = unix::bind(&path).await.unwrap();
    assert_eq!(listener.path(), path);
    assert_eq!(
        unix::bind(&path).await.err().unwrap().kind(),
        io::ErrorKind::AddrInUse
    );
    drop(listener);
    assert!(path.exists());
    assert!(
        UnixStream::connect(&path).is_err(),
        "drop must close the listener"
    );
    let file = dir.path().join("existing");
    std::fs::write(&file, b"keep").unwrap();
    assert!(unix::bind(&file).await.is_err());
    assert_eq!(std::fs::read(&file).unwrap(), b"keep");
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    assert!(unix::bind(&link).await.is_err());
    assert!(
        std::fs::symlink_metadata(link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let error = unix::bind(dir.path().join("missing/child"))
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.raw_os_error().is_some());
}
#[simple_server::test]
async fn nested_routes_stream_both_directions_with_metadata_and_trailers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stream.sock");
    let listener = unix::bind(&path).await.unwrap();
    let router = Router::new()
        .unwrap()
        .nest(
            "/api",
            Router::new()
                .unwrap()
                .route(
                    "/echo/{id}",
                    MethodRouter::new()
                        .unwrap()
                        .on(Method::POST, |request| async move {
                            let metadata =
                                request.extensions().get::<e::RequestMetadata>().unwrap();
                            assert!(metadata.peer.is_none());
                            assert_eq!(metadata.path_params, [("id".to_owned(), "a/b".to_owned())]);
                            assert_eq!(
                                metadata.original_uri.as_ref().unwrap(),
                                "/api/echo/a%2Fb?q=a%2Fb"
                            );
                            assert_eq!(request.headers()["host"], "original.example");
                            assert_eq!(request.headers().get_all("x-repeat").iter().count(), 2);
                            let mut response = Response::new(request.into_body());
                            *response.status_mut() = e::StatusCode::CREATED;
                            response
                                .headers_mut()
                                .append("x-repeat", "one".parse().unwrap());
                            response
                                .headers_mut()
                                .append("x-repeat", "two".parse().unwrap());
                            response
                                .headers_mut()
                                .insert("trailer", "x-end".parse().unwrap());
                            response
                        })
                        .unwrap(),
                )
                .unwrap(),
        )
        .unwrap();
    let stop = Shutdown::new();
    let server = spawn(unix::serve(listener, router, stop.clone()));
    let output = spawn_blocking(move || {
        let mut socket = connect(&path);
        socket.write_all(b"POST /api/echo/a%2Fb?q=a%2Fb HTTP/1.1\r\nHost: original.example\r\nX-Repeat: one\r\nX-Repeat: two\r\nTE: trailers\r\nTransfer-Encoding: chunked\r\nTrailer: x-end\r\nConnection: close\r\n\r\n5\r\nfirst\r\n").unwrap();
        let mut output = Vec::new();
        while !output.windows(5).any(|v| v == b"first") {
            let mut buffer = [0; 1024]; let n = socket.read(&mut buffer).unwrap(); assert!(n > 0); output.extend_from_slice(&buffer[..n]);
        }
        // The response arrives before the request has finished uploading.
        socket.write_all(b"4\r\nlast\r\n0\r\nx-end: yes\r\n\r\n").unwrap();
        socket.read_to_end(&mut output).unwrap();
        String::from_utf8(output).unwrap()
    }).await.unwrap();
    assert!(output.starts_with("HTTP/1.1 201"), "{output}");
    assert!(output.contains("x-repeat: one\r\n") && output.contains("x-repeat: two\r\n"));
    assert!(
        output.contains("last") && output.contains("x-end: yes"),
        "{output}"
    );
    finish(stop, server).await;
    assert!(dir.path().join("stream.sock").exists());
}
#[simple_server::test]
async fn pre_requested_shutdown_accepts_no_queued_request_and_leaves_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stop.sock");
    let listener = unix::bind(&path).await.unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let router = Router::new()
        .unwrap()
        .fallback(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            async { Response::new(Body::empty()) }
        })
        .unwrap();
    let mut socket = connect(&path);
    socket
        .write_all(b"GET / HTTP/1.1\r\nHost: local\r\n\r\n")
        .unwrap();
    let stop = Shutdown::new();
    stop.request();
    timeout(Duration::from_secs(1), unix::serve(listener, router, stop))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed.load(Ordering::SeqCst), 0);
    assert!(path.exists());
    assert!(UnixStream::connect(&path).is_err());
}
struct Guard(Arc<AtomicBool>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
#[simple_server::test]
async fn shutdown_drains_live_stream_and_disconnect_releases_pending_body() {
    use futures_util::StreamExt;
    for disconnect in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("drain.sock");
        let listener = unix::bind(&path).await.unwrap();
        let release = Shutdown::new();
        let gate = release.clone();
        let dropped = Arc::new(AtomicBool::new(false));
        let observed = dropped.clone();
        let router = Router::new()
            .unwrap()
            .fallback(move |_| {
                let gate = gate.clone();
                let guard = Guard(dropped.clone());
                async move {
                    let first = futures_util::stream::iter([Ok::<_, io::Error>(
                        e::Bytes::from_static(b"first"),
                    )]);
                    let last = futures_util::stream::once(async move {
                        let _guard = guard;
                        gate.requested().await;
                        Ok::<_, io::Error>(e::Bytes::from_static(b"last"))
                    });
                    Response::new(Body::from_stream(first.chain(last)))
                }
            })
            .unwrap();
        let stop = Shutdown::new();
        let mut server = spawn(unix::serve(listener, router, stop.clone()));
        let mut socket = spawn_blocking(move || {
            let mut socket = connect(&path);
            socket
                .write_all(b"GET / HTTP/1.1\r\nHost: local\r\nConnection: close\r\n\r\n")
                .unwrap();
            let mut output = Vec::new();
            while !output.windows(5).any(|v| v == b"first") {
                let mut buffer = [0; 1024];
                let n = socket.read(&mut buffer).unwrap();
                assert!(n > 0);
                output.extend_from_slice(&buffer[..n]);
            }
            socket
        })
        .await
        .unwrap();
        assert!(!observed.load(Ordering::SeqCst));
        if disconnect {
            drop(socket);
        } else {
            stop.request();
            assert!(
                timeout(Duration::from_millis(20), &mut server)
                    .await
                    .is_err()
            );
            release.request();
            let output = spawn_blocking(move || {
                let mut bytes = Vec::new();
                socket.read_to_end(&mut bytes).unwrap();
                bytes
            })
            .await
            .unwrap();
            assert!(output.windows(4).any(|v| v == b"last"));
        }
        timeout(Duration::from_secs(3), async {
            while !observed.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        finish(stop, server).await;
    }
}
#[cfg(feature = "unix-http")]
#[simple_server::test]
async fn unix_status_headers_methods_and_paths_match_source() {
    use simple_server::web as w;
    use std::path::PathBuf;
    let dir = tempfile::tempdir().unwrap();
    let engine_path = dir.path().join("engine.sock");
    let source_path = dir.path().join("source.sock");
    let router = Router::new()
        .unwrap()
        .route(
            "/items/{id}",
            MethodRouter::new()
                .unwrap()
                .on_handler(Method::GET, |request: e::Request| async move {
                    format!("{} {}", request.method(), request.uri())
                })
                .unwrap(),
        )
        .unwrap();
    let stop = Shutdown::new();
    let server = spawn(unix::serve(
        unix::bind(&engine_path).await.unwrap(),
        router,
        stop.clone(),
    ));
    let listener = std::os::unix::net::UnixListener::bind(&source_path).unwrap();
    let source_stop = simple_server::lifecycle::Shutdown::new();
    let shutdown = source_stop.clone();
    let source = spawn_blocking(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let router = w::Router::new().route(
                    "/items/{id}",
                    w::routing::get(|request: w::Request| async move {
                        format!("{} {}", request.method(), request.uri())
                    }),
                );
                w::unix::serve(
                    simple_server::net::UnixListener::from_std(listener).unwrap(),
                    router,
                    shutdown,
                )
                .await
            })
    });
    fn exchange(path: PathBuf, method: &str, uri: &str) -> (Vec<String>, Vec<u8>) {
        let mut socket = connect(&path);
        write!(
            socket,
            "{method} {uri} HTTP/1.1\r\nHost: local\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).unwrap();
        let split = bytes.windows(4).position(|v| v == b"\r\n\r\n").unwrap();
        let mut headers: Vec<_> = std::str::from_utf8(&bytes[..split])
            .unwrap()
            .split("\r\n")
            .filter(|v| !v.starts_with("date:"))
            .map(str::to_owned)
            .collect();
        headers.sort();
        (headers, bytes[split + 4..].to_vec())
    }
    spawn_blocking(move || {
        for (method, uri) in [
            ("GET", "/items/a%2Fb?q=x%20y"),
            ("HEAD", "/items/a"),
            ("POST", "/items/a"),
            ("GET", "/missing"),
        ] {
            assert_eq!(
                exchange(engine_path.clone(), method, uri),
                exchange(source_path.clone(), method, uri),
                "{method} {uri}"
            );
        }
    })
    .await
    .unwrap();
    finish(stop, server).await;
    source_stop.request();
    timeout(Duration::from_secs(3), source)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
