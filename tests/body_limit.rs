#![cfg(feature = "body-limit")]
use simple_server::{
    axum::{
        Json, Router,
        body::{Body, Bytes, to_bytes},
        extract::{DefaultBodyLimit, Request},
        http::{StatusCode, header},
        response::Response,
        routing::post,
    },
    body_limit::BodyLimit,
};
use std::convert::Infallible;
use tower::ServiceExt;

fn router(shared: bool, limit: usize) -> Router {
    let router = Router::new()
        .route("/bytes", post(|body: Bytes| async move { body }))
        .route(
            "/json",
            post(|Json(body): Json<serde_json::Value>| async move { Json(body) }),
        );
    if shared {
        router.layer(BodyLimit::max(limit))
    } else {
        router.layer(DefaultBodyLimit::max(limit))
    }
}
async fn result(
    app: Router,
    path: &str,
    bytes: Vec<u8>,
    declared: bool,
) -> (StatusCode, simple_server::axum::http::HeaderMap, Bytes) {
    let mut req = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if declared {
        req = req.header(header::CONTENT_LENGTH, bytes.len());
    }
    let chunks: Vec<_> = bytes
        .chunks(3)
        .map(|c| Ok::<_, Infallible>(Bytes::copy_from_slice(c)))
        .collect();
    let response = app
        .oneshot(
            req.body(Body::from_stream(futures_util::stream::iter(chunks)))
                .unwrap(),
        )
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        to_bytes(body, usize::MAX).await.unwrap(),
    )
}
#[tokio::test]
async fn bytes_and_json_match_existing_rejections_at_boundaries() {
    for declared in [false, true] {
        for limit in [0, 8, 9, 10] {
            for payload in [
                Vec::new(),
                b"12345678".to_vec(),
                b"123456789".to_vec(),
                b"1234567890".to_vec(),
                b"{bad json".to_vec(),
            ] {
                for path in ["/bytes", "/json"] {
                    let old = result(router(false, limit), path, payload.clone(), declared).await;
                    let new = result(router(true, limit), path, payload.clone(), declared).await;
                    assert_eq!(
                        old,
                        new,
                        "{path}, limit={limit}, length={}, declared={declared}",
                        payload.len()
                    );
                    if path == "/bytes" {
                        assert_eq!(
                            new.0,
                            if payload.len() > limit {
                                StatusCode::PAYLOAD_TOO_LARGE
                            } else {
                                StatusCode::OK
                            }
                        );
                    }
                }
            }
        }
    }
}
#[tokio::test]
async fn route_overrides_and_unlimited_raw_reads_keep_existing_semantics() {
    for shared in [false, true] {
        let small = router(shared, 4);
        let large = router(shared, 16);
        let app = Router::new().nest("/small", small).nest("/large", large);
        assert_eq!(
            result(app.clone(), "/small/bytes", vec![b'x'; 8], false)
                .await
                .0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            result(app, "/large/bytes", vec![b'x'; 8], false).await.0,
            StatusCode::OK
        );
    }
    let raw = Router::new()
        .route(
            "/bytes",
            post(
                |request: Request| async move { to_bytes(request.into_body(), 100).await.unwrap() },
            ),
        )
        .layer(BodyLimit::max(0));
    assert_eq!(
        result(raw, "/bytes", vec![b'x'; 8], false).await.0,
        StatusCode::OK
    );
}
#[tokio::test]
async fn response_stream_is_not_polled_or_limited_by_request_limit() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let polls = Arc::new(AtomicUsize::new(0));
    let counter = polls.clone();
    let app = Router::new()
        .route(
            "/",
            post(move || async move {
                let body = Body::from_stream(futures_util::stream::once(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, Infallible>(Bytes::from_static(b"long response"))
                }));
                let mut response = Response::new(body);
                response
                    .headers_mut()
                    .insert("x-preserved", "yes".parse().unwrap());
                response.extensions_mut().insert(42usize);
                response
            }),
        )
        .layer(BodyLimit::max(0));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(response.headers()["x-preserved"], "yes");
    assert_eq!(response.extensions().get::<usize>(), Some(&42));
    assert_eq!(
        to_bytes(response.into_body(), 100).await.unwrap(),
        "long response"
    );
    assert_eq!(polls.load(Ordering::SeqCst), 1);
}
#[cfg(feature = "multipart")]
#[tokio::test]
async fn multipart_limits_preserve_rejections_without_content_length() {
    use simple_server::axum::{extract::Multipart, response::IntoResponse};
    async fn upload(mut multipart: Multipart) -> Response {
        match multipart.next_field().await {
            Ok(Some(field)) => match field.bytes().await {
                Ok(bytes) => bytes.into_response(),
                Err(error) => error.into_response(),
            },
            Ok(None) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => error.into_response(),
        }
    }
    let payload = b"--BOUNDARY\r\nContent-Disposition: form-data; name=\"file\"; filename=\"x\"\r\n\r\nhello\r\n--BOUNDARY--\r\n";
    for limit in [payload.len() - 1, payload.len(), payload.len() + 1] {
        let mut outputs = Vec::new();
        for shared in [false, true] {
            let app = Router::new().route("/", post(upload));
            let app = if shared {
                app.layer(BodyLimit::max(limit))
            } else {
                app.layer(DefaultBodyLimit::max(limit))
            };
            let chunks: Vec<_> = payload
                .chunks(7)
                .map(|c| Ok::<_, Infallible>(Bytes::copy_from_slice(c)))
                .collect();
            let req = Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    header::CONTENT_TYPE,
                    "multipart/form-data; boundary=BOUNDARY",
                )
                .body(Body::from_stream(futures_util::stream::iter(chunks)))
                .unwrap();
            let response = app.oneshot(req).await.unwrap();
            let (parts, body) = response.into_parts();
            outputs.push((
                parts.status,
                parts.headers,
                to_bytes(body, 10000).await.unwrap(),
            ));
        }
        assert_eq!(outputs[0], outputs[1]);
        assert_eq!(
            outputs[1].0,
            if limit < payload.len() {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::OK
            }
        );
    }
}
