#![cfg(all(
    feature = "web",
    feature = "multipart",
    feature = "multipart-owned",
    feature = "body-limit"
))]
use simple_server::web::{self, Body, FromRequest, IntoResponse, Request, Router};
use tower::ServiceExt;
use web::multipart::{Multipart, OwnedMultipart};

fn payload() -> Vec<u8> {
    b"--test\r\nContent-Disposition: form-data; name=\"meta\"\r\n\r\nhello\r\n--test\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.bin\"\r\nContent-Type: application/octet-stream\r\nX-Metadata: retained\r\n\r\n\x00\xffbinary\r\n--test--\r\n".to_vec()
}
fn request(content_type: &str, body: Vec<u8>) -> Request {
    Request::builder()
        .method("POST")
        .uri("/")
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn borrowed_fields_match_backend_metadata_bytes_and_errors() {
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
    let new = Router::new()
        .route("/", web::routing::post(shared))
        .layer(simple_server::body_limit::BodyLimit::max(512));
    let old = axum::Router::new()
        .route("/", axum::routing::post(legacy))
        .layer(simple_server::body_limit::BodyLimit::max(512));
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
        let expected = old
            .clone()
            .oneshot(request(content_type, data.clone()))
            .await
            .unwrap();
        let actual = new
            .clone()
            .oneshot(request(content_type, data))
            .await
            .unwrap();
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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
    let request = Request::builder()
        .header("content-type", "multipart/form-data; boundary=test")
        .body(Body::from_stream(stream))
        .unwrap();
    let mut upload = Multipart::from_request(request, &()).await.unwrap();
    let mut field = tokio::time::timeout(std::time::Duration::from_secs(2), upload.next_field())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let chunk = tokio::time::timeout(std::time::Duration::from_secs(2), field.chunk())
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

#[tokio::test]
async fn raw_query_retains_encoding_duplicates_and_empty_query() {
    let app = Router::new().route(
        "/",
        web::routing::get(|web::RawQuery(value): web::RawQuery| async { web::Json(value) }),
    );
    for (uri, expected) in [
        ("/", None),
        ("/?", Some("")),
        ("/?x=a%2Fb&x=a+b", Some("x=a%2Fb&x=a+b")),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let actual: Option<String> =
            serde_json::from_slice(&response.into_body().collect(1024).await.unwrap()).unwrap();
        assert_eq!(actual.as_deref(), expected);
    }
}
