use serde_json::{Value, json};
use simple_server_sys::{
    Callback, Operation, Reply, Resource, Runtime, frame, resource_new, spawn, spawn_blocking,
    timeout, unframe,
};
use std::{
    future::Future,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

#[derive(Clone, Default)]
struct Signal(Arc<SignalState>);
#[derive(Default)]
struct SignalState {
    set: AtomicBool,
    wake: Mutex<Option<Waker>>,
}
impl Signal {
    fn notify(&self) {
        self.0.set.store(true, Ordering::SeqCst);
        let wake = self.0.wake.lock().unwrap().take();
        if let Some(wake) = wake {
            wake.wake();
        }
    }
}
impl Future for Signal {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut wake = self.0.wake.lock().unwrap();
        if self.0.set.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            *wake = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
fn encode(header: Value, body: &[u8]) -> Vec<u8> {
    frame(&serde_json::to_vec(&header).unwrap(), body).unwrap()
}
fn decode(bytes: &[u8]) -> (Value, Vec<u8>) {
    let (header, body) = unframe(bytes).unwrap();
    (serde_json::from_slice(header).unwrap(), body.to_vec())
}
async fn command(header: Value) -> (Value, Vec<u8>) {
    decode(&Operation::new(&encode(header, &[])).unwrap().await.unwrap())
}
async fn bind() -> (Resource, SocketAddr) {
    let (header, _) = command(json!({"op":"server_bind","address":"127.0.0.1:0"})).await;
    assert_eq!(header["ok"], true, "{header}");
    (
        Resource::new(7, header["id"].as_u64().unwrap()),
        header["address"].as_str().unwrap().parse().unwrap(),
    )
}
fn serve(
    listener: &Resource,
    handler: &Callback,
    stop: Signal,
) -> simple_server_sys::JoinHandle<()> {
    let shutdown = Callback::new(move |_| {
        let stop = stop.clone();
        async move {
            stop.await;
            Vec::new()
        }
    })
    .unwrap();
    let operation = Operation::new(&encode(json!({"op":"server_serve","listener":listener.id(),"handler":handler.id(),"shutdown":shutdown.id()}), &[])).unwrap();
    // Both callbacks may be released by the host after operation construction.
    drop(shutdown);
    spawn(async move {
        let (result, _) = decode(&operation.await.unwrap());
        assert_eq!(result["ok"], true, "{result}");
    })
}
fn connect(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
}
fn read_headers(stream: &mut TcpStream) -> Vec<u8> {
    let mut headers = Vec::new();
    let mut byte = [0];
    while !headers.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        headers.push(byte[0]);
        assert!(headers.len() < 65536);
    }
    headers
}
struct Dropped(Arc<AtomicUsize>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn streaming_echo_preserves_binary_frames_trailers_and_repeated_headers() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let stop = Signal::default();
        let drops = Arc::new(AtomicUsize::new(0));
        let callback_drops = drops.clone();
        let body_ids = Arc::new(Mutex::new(Vec::new()));
        let recorded_ids = body_ids.clone();
        let (headers_seen, receive_headers) = std::sync::mpsc::channel();
        let handler = Callback::with_reply(move |input| {
            let seen = headers_seen.clone();
            let drops = callback_drops.clone();
            let recorded_ids = recorded_ids.clone();
            async move {
                let (request, _) = decode(&input);
                assert_eq!(request["method"], "POST");
                assert_eq!(request["uri"], "/echo?q=a%20b");
                assert!(request["peer"].as_str().unwrap().starts_with("127.0.0.1:"));
                let clone = resource_new(&encode(json!({"op":"server_body_clone","body":request["body"]}), &[])).unwrap();
                let body = Resource::new(6, decode(&clone).0["id"].as_u64().unwrap());
                recorded_ids.lock().unwrap().extend([request["body"].as_u64().unwrap(), body.id()]);
                // Receiving request headers does not consume its body.
                seen.send(()).unwrap();
                let guard = Arc::new(Dropped(drops));
                let output = Callback::new(move |_| {
                    let body = body.clone();
                    let guard = guard.clone();
                    async move {
                        let _guard = guard;
                        Operation::new(&encode(json!({"op":"server_body_frame","body":body.id()}), &[])).unwrap().await.unwrap()
                    }
                }).unwrap();
                let headers = json!([
                    ["set-cookie", b"one=1".as_slice()], ["set-cookie", b"two=2".as_slice()],
                    ["trailer", b"x-finished".as_slice()], ["x-binary", [128, 255]],
                ]);
                Reply::new(encode(json!({"status":201,"headers":headers,"body":output.id()}), &[])).keep_alive(output)
            }
        }).unwrap();
        let server = serve(&listener, &handler, stop.clone());
        drop(handler);
        let client = spawn_blocking(move || {
            let mut stream = connect(address);
            stream.write_all(b"POST /echo?q=a%20b HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nTrailer: x-finished\r\nTE: trailers\r\nConnection: close\r\n\r\n").unwrap();
            receive_headers.recv_timeout(Duration::from_secs(5)).unwrap();
            stream.write_all(b"3\r\n\x00\x80\xff\r\n").unwrap();
            let headers = read_headers(&mut stream);
            assert!(headers.starts_with(b"HTTP/1.1 201"));
            assert_eq!(String::from_utf8_lossy(&headers).matches("set-cookie:").count(), 2);
            assert!(headers.windows(14).any(|value| value == b"x-binary: \x80\xff\r\n"));
            let mut first = [0_u8; 8];
            stream.read_exact(&mut first).unwrap();
            assert_eq!(&first, b"3\r\n\x00\x80\xff\r\n");
            // The response has streamed before the remainder of the request exists.
            stream.write_all(b"3\r\ntwo\r\n0\r\nx-finished: yes\r\n\r\n").unwrap();
            let mut rest = Vec::new();
            stream.read_to_end(&mut rest).unwrap();
            assert_eq!(rest, b"3\r\ntwo\r\n0\r\nx-finished: yes\r\n\r\n");
        });
        timeout(Duration::from_secs(10), client).await.unwrap().unwrap();
        stop.notify();
        timeout(Duration::from_secs(5), server).await.unwrap().unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        for id in body_ids.lock().unwrap().iter() {
            assert!(resource_new(&encode(json!({"op":"server_body_clone","body":id}), &[])).is_err(), "completed request retained a body registration");
        }
    });
}

#[test]
fn shutdown_drains_an_in_flight_stream() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let stop = Signal::default();
        let finish_body = Signal::default();
        let finish = finish_body.clone();
        let (started, receive_started) = std::sync::mpsc::channel();
        let handler = Callback::with_reply(move |_| {
            let finish = finish.clone();
            let started = started.clone();
            async move {
                let output = Callback::new(move |_| {
                    let finish = finish.clone();
                    let started = started.clone();
                    async move {
                        started.send(()).unwrap();
                        finish.await;
                        encode(json!({"kind":"end"}), &[])
                    }
                })
                .unwrap();
                Reply::new(encode(
                    json!({"status":200,"headers":[],"body":output.id()}),
                    &[],
                ))
                .keep_alive(output)
            }
        })
        .unwrap();
        let server = serve(&listener, &handler, stop.clone());
        let client = spawn_blocking(move || {
            let mut stream = connect(address);
            stream
                .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .unwrap();
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).unwrap();
            bytes
        });
        spawn_blocking(move || {
            receive_started
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
        })
        .await
        .unwrap();
        stop.notify();
        simple_server_sys::sleep(Duration::from_millis(20)).await;
        assert!(
            !server.is_finished(),
            "shutdown must wait for an in-flight body"
        );
        finish_body.notify();
        let bytes = timeout(Duration::from_secs(5), client)
            .await
            .unwrap()
            .unwrap();
        assert!(bytes.starts_with(b"HTTP/1.1 200"));
        timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
    });
}

#[test]
fn shutdown_before_serve_releases_listener_without_dispatching() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let handler =
            Callback::new(|_| async { panic!("must not dispatch after shutdown") }).unwrap();
        let stop = Signal::default();
        stop.notify();
        serve(&listener, &handler, stop).await.unwrap();
        assert!(TcpStream::connect(address).is_err());
    });
}

#[test]
fn disconnect_cancels_pending_host_body_and_releases_captures() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let drops = Arc::new(AtomicUsize::new(0));
        let count = drops.clone();
        let (started, receive_started) = std::sync::mpsc::channel();
        let handler = Callback::with_reply(move |_| {
            let count = count.clone();
            let started = started.clone();
            async move {
                let output = Callback::new(move |_| {
                    let started = started.clone();
                    let guard = Dropped(count.clone());
                    async move {
                        let _guard = guard;
                        started.send(()).unwrap();
                        std::future::pending().await
                    }
                })
                .unwrap();
                Reply::new(encode(
                    json!({"status":200,"headers":[],"body":output.id()}),
                    &[],
                ))
                .keep_alive(output)
            }
        })
        .unwrap();
        let stop = Signal::default();
        let server = serve(&listener, &handler, stop.clone());
        spawn_blocking(move || {
            let mut stream = connect(address);
            stream
                .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
            receive_started
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            socket2::SockRef::from(&stream)
                .set_linger(Some(Duration::ZERO))
                .unwrap();
            drop(stream); // Send RST, not a legal HTTP write-half close.
        })
        .await
        .unwrap();
        timeout(Duration::from_secs(5), async {
            while drops.load(Ordering::SeqCst) == 0 {
                simple_server_sys::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        stop.notify();
        timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
    });
}

fn resource(kind: u32, command: Value) -> Resource {
    let output = resource_new(&encode(command, &[])).unwrap();
    Resource::new(kind, decode(&output).0["id"].as_u64().unwrap())
}
fn request(address: SocketAddr, method: &str, path: &str) -> Vec<u8> {
    let mut stream = connect(address);
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    response
}

#[test]
fn engine_router_preserves_nested_paths_head_and_method_fallbacks() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let handler = Callback::new(|input| async move {
            let (request, _) = decode(&input);
            assert_eq!(request["matched_path"], "/api/users/{name}");
            assert_eq!(request["uri"], "/users/alice%20smith?q=1");
            assert_eq!(request["original_uri"], "/api/users/alice%20smith?q=1");
            assert_eq!(request["path_params"], json!([["name", "alice smith"]]));
            encode(json!({"status":200,"headers":[]}), b"hello")
        }).unwrap();
        let methods = resource(9, json!({"op":"method_new"}));
        let methods = resource(9, json!({"op":"method_handler","methods":methods.id(),"method":"GET","handler":handler.id()}));
        let nested = resource(8, json!({"op":"router_new"}));
        let nested = resource(8, json!({"op":"router_route","router":nested.id(),"path":"/users/{name}","methods":methods.id()}));
        let router = resource(8, json!({"op":"router_new"}));
        let router = resource(8, json!({"op":"router_nest","router":router.id(),"path":"/api","other":nested.id()}));
        let stop = Signal::default(); let shutdown_signal = stop.clone();
        let shutdown = Callback::new(move |_| { let stop = shutdown_signal.clone(); async move { stop.await; Vec::new() } }).unwrap();
        let operation = Operation::new(&encode(json!({"op":"server_serve","listener":listener.id(),"router":router.id(),"shutdown":shutdown.id()}), &[])).unwrap();
        // Router and method resources own callback references independently.
        drop(handler); drop(methods); drop(nested); drop(router); drop(shutdown);
        let server = spawn(operation);
        spawn_blocking(move || {
            let get = request(address, "GET", "/api/users/alice%20smith?q=1");
            assert!(get.starts_with(b"HTTP/1.1 200")); assert!(get.ends_with(b"hello"));
            let head = request(address, "HEAD", "/api/users/alice%20smith?q=1");
            assert!(head.starts_with(b"HTTP/1.1 200")); assert!(head.ends_with(b"\r\n\r\n"));
            assert!(String::from_utf8_lossy(&head).contains("content-length: 5"));
            let post = request(address, "POST", "/api/users/alice%20smith?q=1");
            assert!(post.starts_with(b"HTTP/1.1 405"));
            let post = String::from_utf8_lossy(&post);
            assert!(post.lines().any(|line| line.starts_with("allow:") && line.contains("GET") && line.contains("HEAD")));
            assert!(request(address, "GET", "/missing").starts_with(b"HTTP/1.1 404"));
        }).await.unwrap();
        stop.notify();
        assert_eq!(decode(&timeout(Duration::from_secs(5), server).await.unwrap().unwrap().unwrap()).0["ok"], true);
    });
}

#[test]
fn invalid_replies_and_handler_panics_release_reply_resources_and_return_500() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let drops = Arc::new(AtomicUsize::new(0));
        let count = drops.clone();
        let handler = Callback::with_reply(move |input| {
            let count = count.clone();
            async move {
                let (request, _) = decode(&input);
                let guard = Dropped(count);
                if request["uri"] == "/panic" {
                    panic!("handler failed")
                }
                Reply::new(encode(
                    json!({"status":200,"headers":[["bad\nname", [65]]]}),
                    &[],
                ))
                .keep_alive(guard)
            }
        })
        .unwrap();
        let stop = Signal::default();
        let server = serve(&listener, &handler, stop.clone());
        spawn_blocking(move || {
            for path in ["/invalid", "/panic"] {
                assert!(request(address, "GET", path).starts_with(b"HTTP/1.1 500"));
            }
        })
        .await
        .unwrap();
        stop.notify();
        server.await.unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    });
}

#[test]
fn http2_uses_the_engine_on_both_sides() {
    Runtime::new().unwrap().block_on(async {
        let (listener, address) = bind().await;
        let handler = Callback::new(|input| async move {
            assert_eq!(decode(&input).0["version"], "HTTP/2.0");
            encode(json!({"status":200,"headers":[]}), b"h2")
        }).unwrap();
        let stop = Signal::default();
        let server = serve(&listener, &handler, stop.clone());
        let client = resource(1, json!({"op":"client","no_proxy":true,"http2_prior_knowledge":true}));
        let (response, _) = command(json!({"op":"http_send","client":client.id(),"method":"GET","url":format!("http://{address}/"),"headers":[]})).await;
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["version"], "HTTP/2.0");
        let response = Resource::new(2, response["id"].as_u64().unwrap());
        let (chunk, bytes) = command(json!({"op":"http_chunk","response":response.id()})).await;
        assert_eq!(chunk["ok"], true, "{chunk}");
        assert_eq!(bytes, b"h2");
        drop(response); drop(client);
        stop.notify();
        timeout(Duration::from_secs(5), server).await.unwrap().unwrap();
    });
}
