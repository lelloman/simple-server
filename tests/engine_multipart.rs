#![cfg(feature = "engine-web")]
use simple_server::engine_web::{self as web, Body, FromRequest, Request};
use web::multipart::{Multipart, OwnedMultipart};

fn payload() -> Vec<u8> {
    b"--test\r\nContent-Disposition: form-data; name=\"meta\"\r\n\r\nhello\r\n--test\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.bin\"\r\nContent-Type: application/octet-stream\r\nX-Metadata: retained\r\n\r\n\x00\xffbinary\r\n--test--\r\n".to_vec()
}
fn request(content_type: &str, body: Vec<u8>) -> Request {
    http::Request::builder()
        .method("POST")
        .uri("/")
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap()
}

#[cfg(all(feature = "web", feature = "multipart", feature = "body-limit"))]
#[simple_server::test]
async fn borrowed_fields_match_backend_metadata_bytes_and_errors() {
    use tower::ServiceExt;
    use tower_layer::Layer;
    use web::IntoResponse;
    async fn shared(mut upload: Multipart) -> web::Response {
        let mut out = Vec::new();
        loop {
            let field = match upload.next_field().await {
                Ok(Some(field)) => field,
                Ok(None) => return web::Json(out).into_response(),
                Err(error) => return error.into_response(),
            };
            let info = (
                field.name().map(str::to_owned),
                field.file_name().map(str::to_owned),
                field.content_type().map(str::to_owned),
                field
                    .headers()
                    .get("x-metadata")
                    .map(|v| v.to_str().unwrap().to_owned()),
            );
            match field.bytes().await {
                Ok(bytes) => out.push((info, bytes.to_vec())),
                Err(error) => return error.into_response(),
            }
        }
    }
    async fn legacy(mut upload: axum::extract::Multipart) -> axum::response::Response {
        let mut out = Vec::new();
        loop {
            let field = match upload.next_field().await {
                Ok(Some(field)) => field,
                Ok(None) => return axum::response::IntoResponse::into_response(axum::Json(out)),
                Err(error) => return axum::response::IntoResponse::into_response(error),
            };
            let info = (
                field.name().map(str::to_owned),
                field.file_name().map(str::to_owned),
                field.content_type().map(str::to_owned),
                field
                    .headers()
                    .get("x-metadata")
                    .map(|v| v.to_str().unwrap().to_owned()),
            );
            match field.bytes().await {
                Ok(bytes) => out.push((info, bytes.to_vec())),
                Err(error) => return axum::response::IntoResponse::into_response(error),
            }
        }
    }
    for (content_type, data) in [
        ("multipart/form-data; boundary=test", payload()),
        ("multipart/form-data", vec![]),
        ("application/json", b"{}".to_vec()),
        (
            "multipart/form-data; boundary=test",
            b"--test\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\ntruncated".to_vec(),
        ),
        (
            "multipart/form-data; boundary=test",
            b"--test\r\nnot a header\r\n\r\ndata\r\n--test--\r\n".to_vec(),
        ),
        (
            "multipart/form-data; boundary=test",
            format!(
                "--test\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\n{}\r\n--test--\r\n",
                "x".repeat(1024)
            )
            .into_bytes(),
        ),
    ] {
        let expected = simple_server::body_limit::BodyLimit::max(512)
            .layer(tower::service_fn(|request| async {
                let response = match <axum::extract::Multipart as axum::extract::FromRequest<
                        (),
                    >>::from_request(request, &())
                    .await
                    {
                        Ok(upload) => legacy(upload).await,
                        Err(error) => axum::response::IntoResponse::into_response(error),
                    };
                Ok::<_, std::convert::Infallible>(response)
            }))
            .oneshot(request(content_type, data.clone()).map(axum::body::Body::new))
            .await
            .unwrap();
        let mut request = request(content_type, data);
        request.extensions_mut().insert(web::BodyLimit(512));
        let actual = match Multipart::from_request(request, &()).await {
            Ok(upload) => shared(upload).await,
            Err(error) => error.into_response(),
        };
        assert_eq!(actual.status(), expected.status());
        assert_eq!(actual.headers(), expected.headers());
        assert_eq!(
            actual.into_body().collect(8192).await.unwrap(),
            axum::body::to_bytes(expected.into_body(), 8192)
                .await
                .unwrap()
        );
    }
}

#[simple_server::test]
async fn owned_fields_and_errors_have_shared_types() {
    use futures_util::StreamExt;
    let mut upload = OwnedMultipart::from_request(
        request("multipart/form-data; boundary=test", payload()),
        &(),
    )
    .await
    .unwrap();
    let first = upload.next_field().await.unwrap().unwrap();
    let error = match upload.next_field().await {
        Err(error) => error,
        Ok(_) => panic!("concurrent field must fail"),
    };
    fn shared_error(_: &web::multipart::MultipartError) {}
    shared_error(&error);
    assert!(std::error::Error::source(&error).is_some());
    assert!(!error.body_text().is_empty());
    assert_eq!(first.text().await.unwrap(), "hello");
    // Start a new reader because exclusivity errors need not permit recovery.
    let mut upload = OwnedMultipart::from_request(
        request("multipart/form-data; boundary=test", payload()),
        &(),
    )
    .await
    .unwrap();
    upload
        .next_field()
        .await
        .unwrap()
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let mut field = upload.next_field().await.unwrap().unwrap();
    assert_eq!(field.file_name(), Some("test.bin"));
    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(bytes, b"\x00\xffbinary");
    drop(field);
    assert!(upload.next_field().await.unwrap().is_none());
}

#[simple_server::test]
async fn borrowed_field_streaming_and_text_preserve_content() {
    use futures_util::StreamExt;
    let mut upload = Multipart::from_request(
        request("multipart/form-data; boundary=test", payload()),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(
        upload
            .next_field()
            .await
            .unwrap()
            .unwrap()
            .text()
            .await
            .unwrap(),
        "hello"
    );
    let mut field = upload.next_field().await.unwrap().unwrap();
    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(bytes, b"\x00\xffbinary");
    drop(field);
    assert!(upload.next_field().await.unwrap().is_none());
}

#[simple_server::test]
async fn chunk_read_is_lazy_and_dropping_upload_releases_body() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let stream = futures_util::stream::unfold(
        (Guard(dropped.clone()), false),
        |(guard, sent)| async move {
            if sent {
                std::future::pending::<()>().await;
            }
            let chunk = format!(
                "--test\r\nContent-Disposition: form-data; name=\"file\"\r\n\r\n{}",
                "x".repeat(4096)
            );
            Some((
                Ok::<_, std::io::Error>(web::Bytes::from(chunk)),
                (guard, true),
            ))
        },
    );
    let request = http::Request::builder()
        .header("content-type", "multipart/form-data; boundary=test")
        .body(Body::from_stream(stream))
        .unwrap();
    let mut upload = Multipart::from_request(request, &()).await.unwrap();
    let mut field =
        simple_server::time::timeout(std::time::Duration::from_secs(2), upload.next_field())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    let chunk = simple_server::time::timeout(std::time::Duration::from_secs(2), field.chunk())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!chunk.is_empty());
    assert!(!dropped.load(Ordering::SeqCst));
    drop(field);
    drop(upload);
    assert!(dropped.load(Ordering::SeqCst));
}

#[simple_server::test]
async fn owned_field_outlives_reader_and_dropped_fields_are_skipped() {
    let mut upload = OwnedMultipart::from_request(
        request("multipart/form-data; boundary=test", payload()),
        &(),
    )
    .await
    .unwrap();
    drop(upload.next_field().await.unwrap().unwrap());
    let field = upload.next_field().await.unwrap().unwrap();
    drop(upload);
    assert_eq!(field.bytes().await.unwrap(), b"\x00\xffbinary"[..]);
}

#[simple_server::test]
async fn text_uses_charset_and_replacement_decoding() {
    for (charset, data, expected) in [
        ("iso-8859-1", b"caf\xe9".as_slice(), "café"),
        ("utf-8", b"\xff".as_slice(), "�"),
        ("not-a-charset", b"\xef\xbb\xbfhello".as_slice(), "hello"),
    ] {
        let mut bytes = format!("--test\r\nContent-Disposition: form-data; name=\"text\"\r\nContent-Type: text/plain; charset={charset}\r\n\r\n").into_bytes();
        bytes.extend_from_slice(data);
        bytes.extend_from_slice(b"\r\n--test--\r\n");
        let mut upload = OwnedMultipart::from_request(
            request("multipart/form-data; boundary=\"test\"", bytes),
            &(),
        )
        .await
        .unwrap();
        assert_eq!(
            upload
                .next_field()
                .await
                .unwrap()
                .unwrap()
                .text()
                .await
                .unwrap(),
            expected
        );
    }
}

#[simple_server::test]
async fn cancellation_at_each_parse_stage_releases_the_body() {
    use futures_util::{FutureExt, StreamExt};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Pending(Arc<AtomicBool>);
    impl futures_util::Stream for Pending {
        type Item = Result<web::Bytes, std::io::Error>;
        fn poll_next(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            std::task::Poll::Pending
        }
    }
    impl Drop for Pending {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    for stage in 0..4 {
        let dropped = Arc::new(AtomicBool::new(false));
        let prefix = if stage == 0 {
            "--test\r\n"
        } else {
            "--test\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\n"
        };
        let stream = futures_util::stream::iter([Ok(web::Bytes::from_static(prefix.as_bytes()))])
            .chain(Pending(dropped.clone()));
        let request = http::Request::builder()
            .header("content-type", "multipart/form-data; boundary=test")
            .body(Body::from_stream(stream))
            .unwrap();
        let mut upload = OwnedMultipart::from_request(request, &()).await.unwrap();
        assert!(!dropped.load(Ordering::SeqCst));
        if stage == 0 {
            assert!(upload.next_field().now_or_never().is_none());
        } else {
            let mut field = upload.next_field().await.unwrap().unwrap();
            match stage {
                1 => {
                    assert!(field.chunk().now_or_never().is_none());
                    drop(field);
                }
                2 => {
                    assert!(field.text().now_or_never().is_none());
                }
                3 => {
                    assert!(field.chunk().now_or_never().is_none());
                    assert!(field.text().now_or_never().is_none());
                }
                _ => unreachable!(),
            }
        }
        drop(upload);
        assert!(
            dropped.load(Ordering::SeqCst),
            "stage {stage} leaked a body or native registration"
        );
    }
}

#[simple_server::test]
async fn limits_apply_to_the_entire_upload_and_keep_borrowed_owned_error_texts() {
    let mut data = b"--test\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\n".to_vec();
    data.extend(vec![b'x'; 2 * 1024 * 1024]);
    data.extend_from_slice(b"\r\n--test--\r\n");
    let mut upload = Multipart::from_request(
        request("multipart/form-data; boundary=test", data.clone()),
        &(),
    )
    .await
    .unwrap();
    let error = match upload.next_field().await {
        Err(error) => error,
        _ => panic!("default total limit should reject oversized body frame"),
    };
    assert_eq!(error.status(), web::StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error.body_text(), "Request payload is too large");
    let mut upload = OwnedMultipart::from_request(
        request("multipart/form-data; boundary=test", data.clone()),
        &(),
    )
    .await
    .unwrap();
    let error = match upload.next_field().await {
        Err(error) => error,
        _ => panic!("owned limit"),
    };
    assert_eq!(error.status(), web::StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error.body_text(), "failed to read stream");
    let mut request = request("multipart/form-data; boundary=test", data);
    request
        .extensions_mut()
        .insert(web::BodyLimit(3 * 1024 * 1024));
    let mut upload = Multipart::from_request(request, &()).await.unwrap();
    assert_eq!(
        upload
            .next_field()
            .await
            .unwrap()
            .unwrap()
            .bytes()
            .await
            .unwrap()
            .len(),
        2 * 1024 * 1024
    );
}
