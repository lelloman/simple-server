#![cfg(feature = "web")]
use simple_server::web::{
    self, Body, Extension, IntoResponse, Request, Router, StatusCode, Uri,
    middleware::{self, Next},
    routing::get,
};
use tower::ServiceExt;

#[tokio::test]
async fn nested_standard_service_preserves_prefix_query_and_auth_order() {
    async fn auth(request: Request, next: Next) -> web::Response {
        if !request.headers().contains_key("authorization") {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        next.run(request).await
    }
    let service = tower::service_fn(|request: Request| async move {
        Ok::<_, std::convert::Infallible>(http::Response::new(http_body_util::Full::new(
            web::Bytes::from(request.uri().to_string()),
        )))
    });
    let app = Router::new()
        .nest_service("/mcp", service)
        .route_layer(middleware::from_fn(auth));
    for (path, want) in [
        ("/mcp", "/"),
        ("/mcp/", "/"),
        ("/mcp/tool?value=1", "/tool?value=1"),
    ] {
        let denied = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let allowed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("authorization", "token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.into_body().collect(100).await.unwrap(), want);
    }
    assert_eq!(
        app.oneshot(
            Request::builder()
                .uri("/outside")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn method_fallback_and_uri_extraction_match_backend() {
    let shared = Router::new().fallback_methods(get(|uri: Uri| async move { uri.to_string() }));
    let previous = axum::Router::new().fallback(axum::routing::get(|uri: http::Uri| async move {
        uri.to_string()
    }));
    for method in ["GET", "HEAD", "POST", "OPTIONS"] {
        let request = || {
            http::Request::builder()
                .method(method)
                .uri("/some/path?query=value")
        };
        let response = shared
            .clone()
            .oneshot(request().body(Body::empty()).unwrap())
            .await
            .unwrap();
        let old = previous
            .clone()
            .oneshot(request().body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), old.status());
        assert_eq!(response.headers(), old.headers());
        assert_eq!(
            response.into_body().collect(100).await.unwrap(),
            axum::body::to_bytes(old.into_body(), 100).await.unwrap()
        );
    }
}

#[tokio::test]
async fn optional_extension_distinguishes_present_from_absent() {
    async fn handler(value: Option<Extension<u32>>) -> String {
        value.map_or("absent".into(), |Extension(v)| v.to_string())
    }
    let app = Router::new().route("/", get(handler));
    let absent = app
        .clone()
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
    assert_eq!(absent.into_body().collect(10).await.unwrap(), "absent");
    let present = app
        .layer(Extension(42u32))
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
    assert_eq!(present.into_body().collect(10).await.unwrap(), "42");
}

#[cfg(feature = "correlation")]
#[tokio::test]
async fn correlation_preserves_opaque_http_bodies_and_scoped_identity() {
    use simple_server::correlation::{Correlation, IncomingIds, current_id};
    struct Opaque(&'static str);
    let request = http::Request::builder()
        .header("x-request-id", "caller-42")
        .body(Opaque("request"))
        .unwrap();
    let response = Correlation::new(http::HeaderName::from_static("x-request-id"))
        .incoming_ids(IncomingIds::AcceptValidated)
        .run_http(request, |request| async move {
            assert_eq!(current_id().unwrap().as_str(), "caller-42");
            assert_eq!(request.body().0, "request");
            http::Response::builder()
                .status(418)
                .body(Opaque("response"))
                .unwrap()
        })
        .await;
    assert_eq!(response.status(), 418);
    assert_eq!(response.headers()["x-request-id"], "caller-42");
    assert_eq!(response.body().0, "response");
    assert!(current_id().is_none());
}

#[cfg(feature = "health")]
#[tokio::test]
async fn shared_probe_service_preserves_get_head_and_method_rejection() {
    let service = simple_server::health::Probe::liveness().endpoint(|_| "healthy".into_response());
    let app = Router::new().route("/health", web::routing::get_service(service));
    for (method, status, body) in [
        ("GET", 200, "healthy"),
        ("HEAD", 200, ""),
        ("POST", 405, ""),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if status == 405 {
            assert!(
                response.headers()["allow"]
                    .to_str()
                    .unwrap()
                    .contains("GET")
            );
        }
        assert_eq!(response.into_body().collect(100).await.unwrap(), body);
    }
}
