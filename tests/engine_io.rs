#![cfg(all(feature = "runtime", feature = "client", feature = "process"))]

use simple_server::{client::Client, runtime::Runtime};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    time::Duration,
};

#[test]
fn response_is_lazy_and_preserves_repeated_headers_and_binary_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (send, receive) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        assert!(String::from_utf8_lossy(&request).starts_with("GET /test?q=a+b HTTP/1.1\r\n"));
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\n").unwrap();
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
        stream.write_all(&[0, 1, 128, 255]).unwrap();
    });
    Runtime::new().unwrap().block_on(async {
        let response = Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}/test"))
            .query(&[("q", "a b")])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers().get_all("set-cookie").iter().count(), 2);
        send.send(()).unwrap();
        assert_eq!(&response.bytes().await.unwrap()[..], &[0, 1, 128, 255]);
    });
    server.join().unwrap();
}

#[simple_server::test]
async fn process_preserves_exit_status_stdout_and_stderr() {
    let output = simple_server::process::Command::new("/bin/sh")
        .args(["-c", "printf out; printf err >&2; exit 7"])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"out");
    assert_eq!(output.stderr, b"err");
    let error = simple_server::process::Command::new("/definitely-missing-engine-test-command")
        .output()
        .await
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn text_decoding_remains_owned_by_the_engine_client() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=windows-1252\r\nContent-Length: 1\r\nConnection: close\r\n\r\n\x80").unwrap();
    });
    Runtime::new().unwrap().block_on(async {
        assert_eq!(
            Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .get(format!("http://{address}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "€"
        );
    });
    server.join().unwrap();
}
