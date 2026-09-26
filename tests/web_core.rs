#![cfg(feature = "web")]

use serde::{Deserialize, Serialize};
use simple_server::web::{
    self, Body, FromRequestParts, FromState, HeaderMap, IntoResponse, Json, Path, Query, Request,
    Response, Router, State, StatusCode,
    routing::{get, post},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

#[derive(Clone)]
struct AppState {
    name: String,
}
struct Name(String);
impl FromState<AppState> for Name {
    fn from_state(state: &AppState) -> Self {
        Self(state.name.clone())
    }
}
#[derive(Deserialize)]
struct Params {
    count: u32,
}
#[derive(Deserialize, Serialize)]
struct Payload {
    value: String,
}

async fn shared_read(
    State(name): State<Name>,
    Path(id): Path<u64>,
    Query(params): Query<Params>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"id": id, "count": params.count, "name":name.0}))
}
async fn shared_write(Json(body): Json<Payload>) -> impl IntoResponse {
    (StatusCode::CREATED, Json(body))
}

fn shared_router() -> Router {
    Router::new()
        .nest(
            "/v1",
            Router::new()
                .route("/items/{id}", get(shared_read).post(shared_write))
                .route("/items/search", get(|| async { "static" })),
        )
        .merge(Router::new().route("/extra", get(|| async { StatusCode::NO_CONTENT })))
        .with_state(AppState {
            name: "service".into(),
        })
}

async fn snapshot(response: Response) -> (StatusCode, HeaderMap, Vec<u8>) {
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect(4 * 1024 * 1024).await.unwrap().to_vec(),
    )
}

#[tokio::test]
async fn routing_extraction_and_errors_match_existing_backend_contract() {
    use simple_server::axum as old;
    async fn read(
        old::extract::State(state): old::extract::State<AppState>,
        old::extract::Path(id): old::extract::Path<u64>,
        old::extract::Query(params): old::extract::Query<Params>,
    ) -> old::Json<serde_json::Value> {
        old::Json(serde_json::json!({"id":id,"count":params.count,"name":state.name}))
    }
    async fn write(old::Json(body): old::Json<Payload>) -> (StatusCode, old::Json<Payload>) {
        (StatusCode::CREATED, old::Json(body))
    }
    let legacy: old::Router = old::Router::new()
        .nest(
            "/v1",
            old::Router::new()
                .route("/items/{id}", old::routing::get(read).post(write))
                .route("/items/search", old::routing::get(|| async { "static" })),
        )
        .merge(old::Router::new().route(
            "/extra",
            old::routing::get(|| async { StatusCode::NO_CONTENT }),
        ))
        .with_state(AppState {
            name: "service".into(),
        });
    for (method, uri, mime, body) in [
        ("GET", "/v1/items/42?count=2", None, "".to_owned()),
        ("HEAD", "/v1/items/42?count=2", None, "".to_owned()),
        ("GET", "/v1/items/42?count=bad", None, "".to_owned()),
        ("GET", "/v1/items/42", None, "".to_owned()),
        ("GET", "/v1/items/nope?count=1", None, "".to_owned()),
        ("GET", "/v1/items/%FF?count=1", None, "".to_owned()),
        ("GET", "/v1/items/search", None, "".to_owned()),
        ("GET", "/missing", None, "".to_owned()),
        ("GET", "/extra", None, "".to_owned()),
        ("DELETE", "/v1/items/42", None, "".to_owned()),
        (
            "POST",
            "/v1/items/42",
            Some("application/json"),
            r#"{"value":"café"}"#.to_owned(),
        ),
        (
            "POST",
            "/v1/items/42",
            Some("application/problem+json"),
            r#"{"value":"ok"}"#.to_owned(),
        ),
        ("POST", "/v1/items/42", Some("text/plain"), "{}".to_owned()),
        ("POST", "/v1/items/42", None, "{}".to_owned()),
        (
            "POST",
            "/v1/items/42",
            Some("application/json"),
            "{".to_owned(),
        ),
        (
            "POST",
            "/v1/items/42",
            Some("application/json"),
            r#"{"value":false}"#.to_owned(),
        ),
        (
            "POST",
            "/v1/items/42",
            Some("application/json"),
            format!(r#"{{"value":"{}"}}"#, "x".repeat(2 * 1024 * 1024)),
        ),
    ] {
        let request = || {
            let mut builder = http::Request::builder().method(method).uri(uri);
            if let Some(mime) = mime {
                builder = builder.header("content-type", mime);
            }
            builder
        };
        let current = shared_router()
            .oneshot(request().body(Body::from(body.clone())).unwrap())
            .await
            .unwrap();
        let previous = legacy
            .clone()
            .oneshot(request().body(old::body::Body::from(body)).unwrap())
            .await
            .unwrap();
        let (parts, body) = previous.into_parts();
        let previous = (
            parts.status,
            parts.headers,
            old::body::to_bytes(body, 4 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        );
        assert_eq!(snapshot(current).await, previous, "{method} {uri} {mime:?}");
    }
}

#[tokio::test]
async fn responses_preserve_status_headers_extensions_and_body() {
    let mut headers = HeaderMap::new();
    headers.append("set-cookie", "a=1".parse().unwrap());
    headers.append("set-cookie", "b=2".parse().unwrap());
    let mut response = (
        StatusCode::ACCEPTED,
        headers,
        Json(serde_json::json!({"ok":true})),
    )
        .into_response();
    response.extensions_mut().insert(42usize);
    let extensions = response.extensions().clone();
    let app = Router::new().route(
        "/",
        get(move || {
            let extensions = extensions.clone();
            async move {
                let mut result = StatusCode::NO_CONTENT.into_response();
                *result.extensions_mut() = extensions;
                result
            }
        }),
    );
    let result = app.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(result.extensions().get::<usize>(), Some(&42));
    // Check tuple conversion separately (the response body is not Clone).
    let mut headers = HeaderMap::new();
    headers.append("set-cookie", "a=1".parse().unwrap());
    headers.append("set-cookie", "b=2".parse().unwrap());
    let (status, headers, body) = snapshot(
        (
            StatusCode::ACCEPTED,
            headers,
            Json(serde_json::json!({"ok":true})),
        )
            .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(headers.get_all("set-cookie").iter().count(), 2);
    assert_eq!(headers["content-type"], "application/json");
    assert_eq!(body, br#"{"ok":true}"#);
}

#[derive(Clone, Default)]
struct Calls(Arc<AtomicUsize>);
struct Gate;
impl FromRequestParts<Calls> for Gate {
    type Rejection = StatusCode;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _: &Calls,
    ) -> Result<Self, Self::Rejection> {
        if !parts.headers.contains_key("authorization") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        parts.extensions.insert(73usize);
        Ok(Self)
    }
}
struct ReadBody(String);
impl web::FromRequest<Calls> for ReadBody {
    type Rejection = StatusCode;
    async fn from_request(request: Request, state: &Calls) -> Result<Self, Self::Rejection> {
        assert_eq!(request.extensions().get::<usize>(), Some(&73));
        state.0.fetch_add(1, Ordering::SeqCst);
        let bytes = request
            .into_body()
            .collect(128)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        Ok(Self(String::from_utf8(bytes.to_vec()).unwrap()))
    }
}

#[tokio::test]
async fn custom_extraction_is_ordered_and_auth_failure_skips_body() {
    async fn handler(web::Extract(_): web::Extract<Gate>, ReadBody(body): ReadBody) -> String {
        body
    }
    let calls = Calls::default();
    let app = Router::new()
        .route("/", post(handler))
        .with_state(calls.clone());
    let denied = app
        .clone()
        .oneshot(Request::new(Body::from("payload")))
        .await
        .unwrap();
    // GET is unsupported and never invokes the handler.
    assert_eq!(denied.status(), StatusCode::METHOD_NOT_ALLOWED);
    let denied = app
        .clone()
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/")
                .body(Body::from("payload"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(calls.0.load(Ordering::SeqCst), 0);
    let allowed = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/")
                .header("authorization", "present")
                .body(Body::from("payload"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot(allowed).await.2, b"payload");
    assert_eq!(calls.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn explicit_rejection_capture_and_fallback_are_application_owned() {
    async fn handler(body: Result<Json<Payload>, web::RejectionResponse>) -> impl IntoResponse {
        match body {
            Ok(Json(body)) => (StatusCode::OK, body.value),
            Err(error) => (error.status(), "custom rejection".to_owned()),
        }
    }
    let app = Router::new()
        .route("/", post(handler))
        .fallback(|| async { (StatusCode::NOT_FOUND, "custom fallback") });
    let result = app
        .clone()
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, _, bytes) = snapshot(result).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(bytes, b"custom rejection");
    let result = app
        .oneshot(
            http::Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot(result).await.2, b"custom fallback");
}

#[tokio::test]
async fn explicit_body_collection_limit_is_enforced() {
    assert_eq!(Body::from("abc").collect(3).await.unwrap(), "abc");
    assert!(Body::from("abcd").collect(3).await.is_err());
}

#[cfg(feature = "web-compat")]
#[tokio::test]
async fn compatibility_boundary_preserves_outer_extractor_body_limit() {
    use simple_server::axum as old;
    let app = web::compat::into_axum_router(Router::new().route("/", post(shared_write)))
        .layer(old::extract::DefaultBodyLimit::max(16));
    let response = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(old::body::Body::from(r#"{"value":"longer than limit"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[cfg(feature = "body-limit")]
#[tokio::test]
async fn shared_body_limit_supports_standalone_router() {
    let app = Router::new()
        .route("/", post(|body: String| async move { body }))
        .body_limit(simple_server::body_limit::BodyLimit::max(3));
    for (body, expected) in [
        ("abc", StatusCode::OK),
        ("abcd", StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let request = web::Request::builder()
            .method("POST")
            .uri("/")
            .body(Body::from(body))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            expected
        );
    }
}

#[tokio::test]
async fn sixteen_argument_handler_uses_only_shared_contracts() {
    #[allow(clippy::too_many_arguments)]
    async fn handler(
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
        _: HeaderMap,
    ) -> &'static str {
        "sixteen"
    }
    let app = Router::new().route("/", get(handler));
    assert_eq!(
        snapshot(app.oneshot(Request::new(Body::empty())).await.unwrap())
            .await
            .2,
        b"sixteen"
    );
}

#[cfg(feature = "lifecycle")]
#[tokio::test]
async fn shared_router_serves_real_http_and_shuts_down() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = simple_server::lifecycle::Shutdown::new();
    let server = tokio::spawn(web::serve(listener, shared_router(), shutdown.clone()));
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket
        .write_all(
            b"GET /v1/items/7?count=2 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await
        .unwrap();
    let mut response = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        socket.read_to_string(&mut response),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains(r#""id":7"#));
    shutdown.request();
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[cfg(feature = "body-limit")]
#[tokio::test]
async fn optional_json_matches_backend_presence_errors_and_limits() {
    use simple_server::axum as old;
    async fn shared(value: Option<Json<Payload>>) -> Json<Option<Payload>> {
        Json(value.map(|v| v.0))
    }
    async fn legacy(value: Option<old::Json<Payload>>) -> old::Json<Option<Payload>> {
        old::Json(value.map(|v| v.0))
    }
    let shared = Router::new()
        .route("/", post(shared))
        .layer(simple_server::body_limit::BodyLimit::max(32));
    let legacy: old::Router = old::Router::new()
        .route("/", old::routing::post(legacy))
        .layer(simple_server::body_limit::BodyLimit::max(32));
    for (mime, body, expected) in [
        (None, "", 200),
        (None, "ignored without content type", 200),
        (Some("application/json"), r#"{"value":"ok"}"#, 200),
        (
            Some("application/vnd.example+json"),
            r#"{"value":"ok"}"#,
            200,
        ),
        (Some("application/json"), "", 400),
        (Some("application/json"), "{", 400),
        (Some("application/json"), r#"{"value":42}"#, 422),
        (Some("text/plain"), "", 415),
        (
            Some("application/json"),
            r#"{"value":"this value exceeds the body limit"}"#,
            413,
        ),
    ] {
        let request = || {
            let mut builder = Request::builder().method("POST").uri("/");
            if let Some(mime) = mime {
                builder = builder.header("content-type", mime);
            }
            builder.body(Body::from(body)).unwrap()
        };
        let actual = snapshot(shared.clone().oneshot(request()).await.unwrap()).await;
        let old_response = legacy.clone().oneshot(request()).await.unwrap();
        let (parts, body) = old_response.into_parts();
        let expected_snapshot = (
            parts.status,
            parts.headers,
            old::body::to_bytes(body, usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        );
        assert_eq!(actual.0.as_u16(), expected, "{mime:?}");
        assert_eq!(actual, expected_snapshot);
    }
}

#[tokio::test]
async fn method_route_layer_preserves_unmatched_method_responses() {
    use simple_server::axum as old;
    async fn gate(_: Request, _: web::middleware::Next) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
    async fn old_gate(_: old::extract::Request, _: old::middleware::Next) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
    let shared = Router::new().route(
        "/",
        get(|| async { "ok" }).route_layer(web::middleware::from_fn(gate)),
    );
    let legacy: old::Router = old::Router::new().route(
        "/",
        old::routing::get(|| async { "ok" }).route_layer(old::middleware::from_fn(old_gate)),
    );
    for (method, uri, expected) in [
        ("GET", "/", 401),
        ("HEAD", "/", 401),
        ("POST", "/", 405),
        ("GET", "/missing", 404),
    ] {
        let request = || {
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap()
        };
        let actual = snapshot(shared.clone().oneshot(request()).await.unwrap()).await;
        let expected_response = legacy
            .clone()
            .oneshot(request())
            .await
            .unwrap()
            .map(Body::new);
        assert_eq!(actual.0.as_u16(), expected);
        assert_eq!(actual, snapshot(expected_response).await);
    }
}

#[tokio::test]
async fn standalone_headers_preserve_empty_responses_and_repeated_values() {
    use simple_server::axum as old;
    let mut headers = HeaderMap::new();
    headers.append("set-cookie", "first=1".parse().unwrap());
    headers.append("set-cookie", "second=2".parse().unwrap());
    headers.insert("upload-offset", "42".parse().unwrap());
    let expected = old::response::IntoResponse::into_response(headers.clone()).map(Body::new);
    assert_eq!(
        snapshot(headers.clone().into_response()).await,
        snapshot(expected).await
    );
    let expected =
        old::response::IntoResponse::into_response((StatusCode::NO_CONTENT, headers.clone()))
            .map(Body::new);
    assert_eq!(
        snapshot((StatusCode::NO_CONTENT, headers).into_response()).await,
        snapshot(expected).await
    );
}

#[tokio::test]
async fn method_extraction_matches_backend_including_head_and_custom_methods() {
    use simple_server::web::http::Method;
    async fn method_response(method: Method) -> impl IntoResponse {
        (
            [("x-method", method.as_str().to_owned())],
            method.to_string(),
        )
    }
    async fn backend_response(method: Method) -> impl simple_server::axum::response::IntoResponse {
        (
            [("x-method", method.as_str().to_owned())],
            method.to_string(),
        )
    }
    let shared = Router::new().route("/method", web::routing::any(method_response));
    let backend = simple_server::axum::Router::new().route(
        "/method",
        simple_server::axum::routing::any(backend_response),
    );
    for method in ["GET", "HEAD", "POST", "CUSTOM"] {
        let actual = shared
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/method")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let expected = backend
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/method")
                    .body(simple_server::axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(actual.headers()["x-method"], method);
        let expected = expected.map(Body::new);
        assert_eq!(snapshot(actual).await, snapshot(expected).await);
    }
}

#[tokio::test]
async fn method_extraction_observes_prior_request_head_mutation() {
    use simple_server::web::http::Method;
    struct Rewrite;
    impl<S: Sync> FromRequestParts<S> for Rewrite {
        type Rejection = std::convert::Infallible;
        async fn from_request_parts(
            parts: &mut web::http::request::Parts,
            _: &S,
        ) -> Result<Self, Self::Rejection> {
            parts.method = Method::PATCH;
            Ok(Self)
        }
    }
    let app = Router::new().route(
        "/method",
        get(|before: Method, _: Rewrite, after: Method| async move { format!("{before}:{after}") }),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/method")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot(response).await.2, b"GET:PATCH");
}
