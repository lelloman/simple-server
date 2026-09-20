#![cfg(feature = "correlation")]

use simple_server::{
    axum::{
        Router,
        body::{Body, Bytes, to_bytes},
        extract::{Extension, Request},
        http::{HeaderName, StatusCode},
        middleware::from_fn_with_state,
        response::{IntoResponse, Response},
        routing::get,
    },
    correlation::{Correlation, IncomingIds, RequestId, current_id, middleware},
};
use tower::ServiceExt;

#[tokio::test]
async fn selected_insert_if_missing_retains_repeated_headers() {
    use simple_server::{
        axum::http::HeaderValue,
        correlation::{HeaderRequestId, Propagation, RequestHeader, ResponseHeader},
    };
    for present in [false, true] {
        let mut req = request(None);
        if present {
            req.headers_mut()
                .append("x-request-id", HeaderValue::from_static("first"));
            req.headers_mut()
                .append("x-request-id", HeaderValue::from_static("second"));
        }
        let response = Correlation::default()
            .run_selected(
                req,
                HeaderRequestId::new(HeaderValue::from_static("selected")),
                Propagation {
                    request_header: RequestHeader::IfMissing,
                    response_header: ResponseHeader::Preserve,
                },
                |req| async move {
                    let values: Vec<_> = req
                        .headers()
                        .get_all("x-request-id")
                        .iter()
                        .map(|v| v.to_str().unwrap())
                        .collect();
                    assert_eq!(
                        values,
                        if present {
                            vec!["first", "second"]
                        } else {
                            vec!["selected"]
                        }
                    );
                    let mut response = Response::new(Body::empty());
                    response
                        .headers_mut()
                        .append("x-request-id", HeaderValue::from_static("override1"));
                    response
                        .headers_mut()
                        .append("x-request-id", HeaderValue::from_static("override2"));
                    response
                },
            )
            .await;
        assert_eq!(response.headers().get_all("x-request-id").iter().count(), 2);
        assert_eq!(
            response
                .extensions()
                .get::<HeaderRequestId>()
                .unwrap()
                .as_header_value(),
            "override1"
        );
    }
}

#[tokio::test]
async fn selected_ids_preserve_application_policy_and_opaque_bytes() {
    use simple_server::{
        axum::http::HeaderValue,
        correlation::{HeaderRequestId, Propagation, current_header_id},
    };
    for value in [
        HeaderValue::from_static(""),
        HeaderValue::from_static("legacy value"),
        HeaderValue::from_static("b12871fb-a9c6-4aac-8c48-82ecdc1ea7e4"),
        HeaderValue::from_str(&"x".repeat(128)).unwrap(),
        HeaderValue::from_bytes(&[0xff]).unwrap(),
    ] {
        let selected = HeaderRequestId::new(value.clone());
        let response = Correlation::default()
            .run_selected(
                request(Some("untouched")),
                selected.clone(),
                Propagation::default(),
                |request| {
                    // Synchronous callback construction also executes inside the scope.
                    assert_eq!(current_header_id(), Some(selected.clone()));
                    assert!(current_id().is_none());
                    assert!(request.extensions().get::<RequestId>().is_none());
                    assert_eq!(
                        request.extensions().get::<HeaderRequestId>(),
                        Some(&selected)
                    );
                    assert_eq!(request.headers()["x-request-id"], "untouched");
                    async {
                        Response::builder()
                            .status(400)
                            .header("x-request-id", "wrong")
                            .body(Body::from("unchanged"))
                            .unwrap()
                    }
                },
            )
            .await;
        assert_eq!(response.headers()["x-request-id"], value);
        assert_eq!(
            response.extensions().get::<HeaderRequestId>(),
            Some(&selected)
        );
        assert!(response.extensions().get::<RequestId>().is_none());
        assert_eq!(
            to_bytes(response.into_body(), 64).await.unwrap(),
            "unchanged"
        );
        assert!(current_header_id().is_none());
    }
}

#[tokio::test]
async fn selected_propagation_preserves_response_overrides_and_restores_outer_scope() {
    use simple_server::{
        axum::http::HeaderValue,
        correlation::{
            HeaderRequestId, Propagation, RequestHeader, ResponseHeader, current_header_id,
        },
    };
    let config = &Correlation::default();
    config
        .run(request(None), |outer| async move {
            let outer_id = current_id().unwrap();
            assert_eq!(
                current_header_id().unwrap().as_header_value(),
                outer_id.as_str()
            );
            for override_value in [None, Some("handler")] {
                let selected = HeaderRequestId::new(HeaderValue::from_static("selected"));
                let mut inner = request(Some("original"));
                inner
                    .extensions_mut()
                    .insert(outer.extensions().get::<RequestId>().unwrap().clone());
                let response = config
                    .run_selected(
                        inner,
                        selected.clone(),
                        Propagation {
                            request_header: RequestHeader::Overwrite,
                            response_header: ResponseHeader::Preserve,
                        },
                        |request| async move {
                            assert!(current_id().is_none());
                            assert_eq!(current_header_id(), Some(selected));
                            assert_eq!(request.headers()["x-request-id"], "selected");
                            assert!(request.extensions().get::<RequestId>().is_none());
                            let mut response = Response::new(Body::empty());
                            if let Some(value) = override_value {
                                response
                                    .headers_mut()
                                    .insert("x-request-id", HeaderValue::from_static(value));
                            }
                            response
                        },
                    )
                    .await;
                assert_eq!(
                    response.headers()["x-request-id"],
                    override_value.unwrap_or("selected")
                );
                assert_eq!(
                    response
                        .extensions()
                        .get::<HeaderRequestId>()
                        .unwrap()
                        .as_header_value(),
                    &response.headers()["x-request-id"]
                );
                assert_eq!(current_id(), Some(outer_id.clone()));
            }
            Response::new(Body::empty())
        })
        .await;
    assert!(current_header_id().is_none());
}

#[test]
fn runs_without_runtime_and_restores_context_after_panic() {
    use futures_util::FutureExt;
    let config = Correlation::default();
    let future = config.run(request(None), |_| async {
        assert!(current_id().is_some());
        panic!("handler panic");
    });
    assert!(
        std::panic::AssertUnwindSafe(future)
            .catch_unwind()
            .now_or_never()
            .unwrap()
            .is_err()
    );
    assert!(current_id().is_none());
    assert!(config.run(request(None), echo).now_or_never().is_some());
}

fn request(value: Option<&str>) -> Request {
    let mut builder = Request::builder();
    if let Some(value) = value {
        builder = builder.header("x-request-id", value);
    }
    builder.body(Body::empty()).unwrap()
}

async fn echo(request: Request) -> Response {
    let id = request.extensions().get::<RequestId>().unwrap();
    assert_eq!(current_id().as_ref(), Some(id));
    (StatusCode::FORBIDDEN, id.to_string()).into_response()
}

#[tokio::test]
async fn default_ignores_caller_and_overwrites_response_id_without_changing_body() {
    let config = Correlation::default();
    let mut ids = std::collections::HashSet::new();
    for _ in 0..100 {
        let response = config
            .run(request(Some("caller")), |req| async {
                assert_eq!(req.headers()["x-request-id"], "caller");
                let mut response = echo(req).await;
                response
                    .headers_mut()
                    .insert("x-request-id", "wrong".parse().unwrap());
                response
            })
            .await;
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(id.len(), 32);
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(ids.insert(id.clone()));
        assert_eq!(
            response.extensions().get::<RequestId>().unwrap().as_str(),
            id
        );
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(to_bytes(response.into_body(), 64).await.unwrap(), id);
    }
    assert!(current_id().is_none());
}

#[tokio::test]
async fn explicit_trust_validates_bounded_values_and_uses_first_header() {
    let config = Correlation::default().incoming_ids(IncomingIds::AcceptValidated);
    for value in ["", "has space", "a,b", "slash/id", &"a".repeat(65)] {
        let response = config.run(request(Some(value)), echo).await;
        assert_ne!(response.headers()["x-request-id"], value);
    }
    for value in ["abc-DEF_123.4:5", &"a".repeat(64)] {
        let mut req = request(Some(value));
        req.headers_mut()
            .append("x-request-id", "second".parse().unwrap());
        let response = config.run(req, echo).await;
        assert_eq!(response.headers()["x-request-id"], value);
    }
    for value in ["é", "line\nbreak", "\0"] {
        assert!(RequestId::parse(value).is_none());
    }
}

#[tokio::test]
async fn router_covers_extractor_rejections_not_found_and_method_not_allowed() {
    let config = Correlation::new(HeaderName::from_static("x-correlation-id"))
        .incoming_ids(IncomingIds::AcceptValidated);
    let app = Router::new()
        .route(
            "/",
            get(|Extension(id): Extension<RequestId>| async move {
                assert_eq!(current_id(), Some(id));
                "ok"
            }),
        )
        .route(
            "/json",
            get(|_: simple_server::axum::Json<serde_json::Value>| async { "ok" }),
        )
        .layer(from_fn_with_state(config, middleware));
    for (uri, method, status) in [
        ("/", "GET", 200),
        ("/missing", "GET", 404),
        ("/", "POST", 405),
        ("/json", "GET", 415),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method(method)
                    .header("x-correlation-id", "router-42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        assert_eq!(response.headers()["x-correlation-id"], "router-42");
    }
}

#[tokio::test]
async fn concurrent_requests_and_spawned_tasks_do_not_share_context() {
    let config = Correlation::default().incoming_ids(IncomingIds::AcceptValidated);
    let handler = |request: Request| async {
        tokio::task::yield_now().await;
        assert!(
            tokio::spawn(async { current_id() })
                .await
                .unwrap()
                .is_none()
        );
        echo(request).await
    };
    let (a, b) = tokio::join!(
        config.run(request(Some("a")), handler),
        config.run(request(Some("b")), handler)
    );
    assert_eq!(a.headers()["x-request-id"], "a");
    assert_eq!(b.headers()["x-request-id"], "b");
    assert!(current_id().is_none());
}

#[tokio::test]
async fn nesting_restores_outer_context_and_cancellation_clears_scope() {
    let config = Correlation::default().incoming_ids(IncomingIds::AcceptValidated);
    config
        .run(request(Some("outer")), |_| async {
            config.run(request(Some("inner")), echo).await;
            assert_eq!(current_id().unwrap().as_str(), "outer");
            let pending = config.run(request(Some("cancelled")), |_| async {
                assert_eq!(current_id().unwrap().as_str(), "cancelled");
                std::future::pending::<Response>().await
            });
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(1), pending)
                    .await
                    .is_err()
            );
            assert_eq!(current_id().unwrap().as_str(), "outer");
            Response::new(Body::empty())
        })
        .await;
    assert!(current_id().is_none());
}

#[tokio::test]
async fn streaming_body_is_not_polled_or_buffered_and_upgrade_is_untouched() {
    let config = Correlation::default();
    let response = config
        .run(request(None), |_| async {
            Response::new(Body::from_stream(futures_util::stream::pending::<
                Result<Bytes, std::convert::Infallible>,
            >()))
        })
        .await;
    assert!(response.headers().contains_key("x-request-id"));
    assert!(current_id().is_none());
    let response = config
        .run(request(None), |_| async {
            Response::builder()
                .status(101)
                .header("upgrade", "websocket")
                .body(Body::empty())
                .unwrap()
        })
        .await;
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(response.headers()["upgrade"], "websocket");
}
