#![cfg(all(unix, feature = "engine-web"))]
use futures_util::{Stream, StreamExt};
use http_body_util::BodyExt;
use simple_server::{
    engine_web::{
        self as e, Body, Request, Response, Router, Shutdown,
        unix::{self, UnixClient, UnixHttpErrorKind},
    },
    runtime::{JoinHandle, spawn, spawn_blocking},
    time::{sleep, timeout},
};
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
async fn start(
    router: Router,
) -> (
    tempfile::TempDir,
    PathBuf,
    Shutdown,
    JoinHandle<io::Result<()>>,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("client.sock");
    let listener = unix::bind(&path).await.unwrap();
    let stop = Shutdown::new();
    let server = spawn(unix::serve(listener, router, stop.clone()));
    (dir, path, stop, server)
}
async fn finish(stop: Shutdown, server: JoinHandle<io::Result<()>>) {
    stop.request();
    timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
async fn until(flag: &AtomicBool) {
    timeout(Duration::from_secs(3), async {
        while !flag.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[simple_server::test]
async fn input_and_transport_errors_remain_distinct() {
    use std::os::unix::ffi::OsStringExt;
    let client = UnixClient::new().unwrap();
    for path in [
        PathBuf::new(),
        PathBuf::from(std::ffi::OsString::from_vec(vec![0xff])),
    ] {
        let error = client
            .request(path, Request::new(Body::empty()))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), UnixHttpErrorKind::InvalidRequest);
        assert!(std::error::Error::source(&error).is_some());
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing");
    let error = client
        .request(
            &path,
            http::Request::builder()
                .uri("*")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind(), UnixHttpErrorKind::InvalidRequest);
    let error = client
        .request(&path, Request::new(Body::empty()))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), UnixHttpErrorKind::Transport);
    assert!(std::error::Error::source(&error).is_some());
}
#[simple_server::test]
async fn client_streams_both_directions_and_preserves_forwarded_headers() {
    let router = Router::new()
        .unwrap()
        .fallback(|request| async move {
            assert_eq!(request.method(), e::Method::POST);
            assert_eq!(request.uri(), "/echo?a=one%20two&b=3");
            assert_eq!(request.headers()["host"], "original.example");
            assert_eq!(request.headers().get_all("x-repeat").iter().count(), 2);
            let mut response = Response::new(request.into_body());
            *response.status_mut() = e::StatusCode::CREATED;
            response
                .headers_mut()
                .append("x-repeat", "first".parse().unwrap());
            response
                .headers_mut()
                .append("x-repeat", "second".parse().unwrap());
            response
        })
        .unwrap();
    let (_dir, path, stop, server) = start(router).await;
    let release = Shutdown::new();
    let gate = release.clone();
    let body = futures_util::stream::iter([Ok::<_, io::Error>(e::Bytes::from_static(b"first"))])
        .chain(futures_util::stream::once(async move {
            gate.requested().await;
            Ok::<_, io::Error>(e::Bytes::from_static(b"second"))
        }));
    let request = http::Request::builder()
        .method("POST")
        .uri("http://ignored.example/echo?a=one%20two&b=3")
        .header("host", "original.example")
        .header("x-repeat", "one")
        .header("x-repeat", "two")
        .body(Body::from_stream(body))
        .unwrap();
    let client = UnixClient::new().unwrap();
    let response = timeout(Duration::from_secs(3), client.request(path, request))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), 201);
    assert_eq!(response.version(), e::Version::HTTP_11);
    assert_eq!(response.headers().get_all("x-repeat").iter().count(), 2);
    let mut body = response.into_body();
    assert_eq!(
        timeout(Duration::from_secs(3), body.frame())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap(),
        "first"
    );
    drop(client); // The response owns its native body independently of the pool handle.
    release.request();
    assert_eq!(body.collect(64).await.unwrap(), "second");
    finish(stop, server).await;
}
#[simple_server::test]
async fn request_and_response_trailers_survive_the_client_boundary() {
    let router = Router::new()
        .unwrap()
        .fallback(|request| async move {
            let mut response = Response::new(request.into_body());
            response
                .headers_mut()
                .insert("trailer", "x-end".parse().unwrap());
            response
        })
        .unwrap();
    let (_dir, path, stop, server) = start(router).await;
    let mut trailers = e::HeaderMap::new();
    trailers.insert("x-end", "yes".parse().unwrap());
    let frames = futures_util::stream::iter([
        Ok::<_, io::Error>(e::Frame::data(e::Bytes::from_static(b"data"))),
        Ok(e::Frame::trailers(trailers)),
    ]);
    let request = http::Request::builder()
        .method("POST")
        .header("te", "trailers")
        .header("trailer", "x-end")
        .body(Body::new(http_body_util::StreamBody::new(frames)))
        .unwrap();
    let response = UnixClient::new()
        .unwrap()
        .request(path, request)
        .await
        .unwrap();
    let collected = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap();
    assert_eq!(collected.trailers().unwrap()["x-end"], "yes");
    assert_eq!(collected.to_bytes(), "data");
    finish(stop, server).await;
}
struct Probe {
    first: bool,
    dropped: Arc<AtomicBool>,
    polled: Arc<AtomicBool>,
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
impl Stream for Probe {
    type Item = Result<e::Bytes, io::Error>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.polled.store(true, Ordering::SeqCst);
        if self.first {
            self.first = false;
            std::task::Poll::Ready(Some(Ok(e::Bytes::from_static(b"first"))))
        } else {
            std::task::Poll::Pending
        }
    }
}
#[simple_server::test]
async fn dropping_response_releases_native_body_and_host_producer() {
    let dropped = Arc::new(AtomicBool::new(false));
    let observed = dropped.clone();
    let router = Router::new()
        .unwrap()
        .fallback(move |_| {
            let dropped = dropped.clone();
            async move {
                Response::new(Body::from_stream(Probe {
                    first: true,
                    dropped,
                    polled: Arc::new(AtomicBool::new(false)),
                }))
            }
        })
        .unwrap();
    let (_dir, path, stop, server) = start(router).await;
    let response = UnixClient::new()
        .unwrap()
        .request(path, Request::new(Body::empty()))
        .await
        .unwrap();
    let mut body = response.into_body();
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "first"
    );
    assert!(!observed.load(Ordering::SeqCst));
    drop(body);
    until(&observed).await;
    finish(stop, server).await;
}
#[simple_server::test]
async fn cancelling_pending_request_releases_upload_producer() {
    let router = Router::new()
        .unwrap()
        .fallback(|request| async move {
            let _ = request.into_body().collect(1024).await;
            Response::new(Body::empty())
        })
        .unwrap();
    let (_dir, path, stop, server) = start(router).await;
    let dropped = Arc::new(AtomicBool::new(false));
    let polled = Arc::new(AtomicBool::new(false));
    let body = Body::from_stream(Probe {
        first: true,
        dropped: dropped.clone(),
        polled: polled.clone(),
    });
    let pending = spawn(async move {
        UnixClient::new()
            .unwrap()
            .request(
                path,
                http::Request::builder().method("POST").body(body).unwrap(),
            )
            .await
    });
    until(&polled).await;
    pending.abort();
    assert!(pending.await.is_err());
    until(&dropped).await;
    finish(stop, server).await;
}
#[simple_server::test]
async fn late_response_errors_are_not_collected_or_hidden() {
    let release = Shutdown::new();
    let gate = release.clone();
    let router = Router::new()
        .unwrap()
        .fallback(move |_| {
            let gate = gate.clone();
            async move {
                let body = futures_util::stream::iter([Ok::<_, io::Error>(e::Bytes::from_static(
                    b"first",
                ))])
                .chain(futures_util::stream::once(async move {
                    gate.requested().await;
                    Err(io::Error::other("upstream failed"))
                }));
                Response::new(Body::from_stream(body))
            }
        })
        .unwrap();
    let (_dir, path, stop, server) = start(router).await;
    let response = UnixClient::new()
        .unwrap()
        .request(path, Request::new(Body::empty()))
        .await
        .unwrap();
    let mut body = response.into_body();
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "first"
    );
    assert!(
        timeout(Duration::from_millis(20), body.frame())
            .await
            .is_err()
    );
    release.request();
    assert!(
        timeout(Duration::from_secs(3), body.frame())
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    finish(stop, server).await;
}
#[simple_server::test]
async fn cloned_clients_reuse_a_connection_and_preserve_request_version_and_length() {
    use std::io::{Read, Write};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pool.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let raw = spawn_blocking(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        for _ in 0..2 {
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let headers = String::from_utf8(request).unwrap();
            assert!(
                headers.starts_with("POST /pool?q=1 HTTP/1.1\r\n"),
                "{headers}"
            );
            assert!(headers.contains("content-length: 3\r\n"), "{headers}");
            assert!(!headers.contains("transfer-encoding:"));
            let mut body = [0; 3];
            socket.read_exact(&mut body).unwrap();
            assert_eq!(&body, b"abc");
            socket.write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\nX-Repeat: one\r\nX-Repeat: two\r\n\r\nok").unwrap();
        }
    });
    let client = UnixClient::new().unwrap();
    for client in [client.clone(), client] {
        let request = http::Request::builder()
            .method("POST")
            .uri("/pool?q=1")
            .version(e::Version::HTTP_11)
            .body(Body::from("abc"))
            .unwrap();
        let response = timeout(Duration::from_secs(3), client.request(&path, request))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), 202);
        assert_eq!(response.headers().get_all("x-repeat").iter().count(), 2);
        assert_eq!(response.into_body().collect(16).await.unwrap(), "ok");
        simple_server::runtime::yield_now().await;
    }
    timeout(Duration::from_secs(3), raw).await.unwrap().unwrap();
}

#[cfg(feature = "unix-http")]
#[simple_server::test]
async fn outbound_requests_and_responses_match_source_client() {
    let router=Router::new().unwrap().fallback(|request|async move {
        let version=format!("{:?}",request.version()); let method=request.method().to_string(); let uri=request.uri().to_string();
        let host=request.headers()["host"].to_str().unwrap().to_owned();
        let repeated=request.headers().get_all("x-repeat").iter().map(|v|v.as_bytes().to_vec()).collect::<Vec<_>>();
        let bytes=request.into_body().collect(1024).await.unwrap();
        let mut response=Response::new(Body::from(serde_json::to_vec(&serde_json::json!({"version":version,"method":method,"uri":uri,"host":host,"headers":repeated,"body":bytes.to_vec()})).unwrap()));
        response.headers_mut().insert("x-version",version.parse().unwrap()); response
    }).unwrap();
    let (_dir, path, stop, server) = start(router).await;
    for (method, version) in [
        ("POST", e::Version::HTTP_11),
        ("POST", e::Version::HTTP_10),
        ("HEAD", e::Version::HTTP_11),
    ] {
        let request = || {
            http::Request::builder()
                .method(method)
                .version(version)
                .uri("http://ignored.example/a%2Fb?x=a+b&x=2")
                .header("host", "original.example")
                .header("x-repeat", "one")
                .header("x-repeat", "two")
        };
        let response = UnixClient::new()
            .unwrap()
            .request(
                &path,
                request().body(Body::from(&b"\0\xffbytes"[..])).unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let actual_version = response.version();
        let mut headers = response.headers().clone();
        headers.remove("date");
        let bytes = response.into_body().collect(2048).await.unwrap();
        let source_path = path.clone();
        let source_request = request()
            .body(simple_server::web::Body::from(&b"\0\xffbytes"[..]))
            .unwrap();
        let expected = spawn_blocking(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let response = simple_server::web::unix::UnixClient::new()
                        .request(source_path, source_request)
                        .await
                        .unwrap();
                    let status = response.status();
                    let version = response.version();
                    let mut headers = response.headers().clone();
                    headers.remove("date");
                    (
                        status,
                        version,
                        headers,
                        response.into_body().collect(2048).await.unwrap(),
                    )
                })
        })
        .await
        .unwrap();
        assert_eq!(
            (status, actual_version, headers, bytes),
            expected,
            "{method} {version:?}"
        );
    }
    finish(stop, server).await;
}
#[simple_server::test]
async fn upload_body_errors_become_transport_errors() {
    let router = Router::new()
        .unwrap()
        .fallback(|request| async move {
            let _ = request.into_body().collect(1024).await;
            Response::new(Body::empty())
        })
        .unwrap();
    let (_dir, path, stop, server) = start(router).await;
    let body = Body::from_stream(futures_util::stream::iter([Err::<e::Bytes, _>(
        io::Error::other("upload failed"),
    )]));
    let error = UnixClient::new()
        .unwrap()
        .request(
            path,
            http::Request::builder().method("POST").body(body).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind(), UnixHttpErrorKind::Transport);
    finish(stop, server).await;
}
