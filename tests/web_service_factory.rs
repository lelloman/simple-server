#![cfg(feature = "web")]
use simple_server::web::{Body, IntoResponse, Request, Router, State, StatusCode, routing::post};
use tower::ServiceExt;

#[tokio::test]
async fn factory_accepts_connection_targets_and_standard_request_bodies() {
    async fn echo(State(prefix): State<String>, body: String) -> String {
        prefix + &body
    }
    let factory = Router::new()
        .route("/", post(echo))
        .with_state("echo:".to_owned())
        .into_make_service();
    for target in [1usize, 2] {
        let router = factory.clone().oneshot(target).await.unwrap();
        let request = Request::builder()
            .method("POST")
            .body(http_body_util::Full::new(bytes::Bytes::from_static(
                b"payload",
            )))
            .unwrap();
        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.into_body().collect(100).await.unwrap(),
            "echo:payload"
        );
    }
}

#[tokio::test]
async fn header_arrays_preserve_cookie_replacement_and_response_metadata() {
    let headers = [
        (
            "set-cookie",
            "token=test; HttpOnly; SameSite=Strict; Path=/",
        ),
        ("content-type", "custom/type"),
    ];
    let mut body = "body".into_response();
    body.extensions_mut().insert(42usize);
    let response = (StatusCode::CREATED, headers, body).into_response();
    let previous =
        axum::response::IntoResponse::into_response((StatusCode::CREATED, headers, "body"));
    assert_eq!(response.status(), previous.status());
    assert_eq!(response.headers(), previous.headers());
    assert_eq!(response.extensions().get::<usize>(), Some(&42));
    assert_eq!(response.into_body().collect(100).await.unwrap(), "body");
    let response = ([("x-value", "first"), ("x-value", "second")], Body::empty()).into_response();
    assert_eq!(response.headers()["x-value"], "second");
    assert_eq!(response.headers().get_all("x-value").iter().count(), 1);
}

#[tokio::test]
async fn invalid_header_arrays_match_backend_error_contract() {
    for headers in [[("invalid name", "value")], [("x-value", "invalid\nvalue")]] {
        let response = (headers, "unused").into_response();
        let previous = axum::response::IntoResponse::into_response((headers, "unused"));
        assert_eq!(response.status(), previous.status());
        assert_eq!(response.headers(), previous.headers());
        assert_eq!(
            response.into_body().collect(1000).await.unwrap(),
            axum::body::to_bytes(previous.into_body(), 1000)
                .await
                .unwrap()
        );
        let response = (StatusCode::ACCEPTED, headers, "unused").into_response();
        let previous =
            axum::response::IntoResponse::into_response((StatusCode::ACCEPTED, headers, "unused"));
        assert_eq!(response.status(), previous.status());
        assert_eq!(
            response.into_body().collect(1000).await.unwrap(),
            axum::body::to_bytes(previous.into_body(), 1000)
                .await
                .unwrap()
        );
    }
}
