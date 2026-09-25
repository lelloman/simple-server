#![cfg(feature = "extract")]

use simple_server::extract::{
    Extract, FromRequestParts, IntoRejectionResponse, Parts, RejectionResponse, StatusCode,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
struct State {
    calls: Arc<AtomicUsize>,
}

#[derive(Debug, PartialEq)]
struct User(usize);

impl FromRequestParts<State> for User {
    type Rejection = RejectionResponse;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        state.calls.fetch_add(1, Ordering::SeqCst);
        // Exercise borrowing across a real suspension point.
        tokio::task::yield_now().await;
        parts.extensions.insert("visited");
        match parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
        {
            Some("valid") => Ok(User(42)),
            Some("busy") => {
                let mut response = StatusCode::SERVICE_UNAVAILABLE.into_rejection_response();
                response
                    .headers_mut()
                    .insert("retry-after", "1".parse().unwrap());
                response
                    .headers_mut()
                    .append("set-cookie", "first=1".parse().unwrap());
                response
                    .headers_mut()
                    .append("set-cookie", "second=2".parse().unwrap());
                response.extensions_mut().insert(73usize);
                *response.body_mut() = b"database busy\0".to_vec();
                Err(response)
            }
            _ => Err(StatusCode::UNAUTHORIZED.into_rejection_response()),
        }
    }
}

// Explicit policy: missing/invalid are anonymous, infrastructure errors survive.
impl FromRequestParts<State> for Option<User> {
    type Rejection = RejectionResponse;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        match User::from_request_parts(parts, state).await {
            Ok(user) => Ok(Some(user)),
            Err(error) if error.status() == StatusCode::UNAUTHORIZED => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[tokio::test]
async fn contract_works_without_http_feature_and_revalidates_each_time() {
    let state = State::default();
    let (mut parts, _) = http::Request::new(()).into_parts();
    assert_eq!(
        Option::<User>::from_request_parts(&mut parts, &state)
            .await
            .unwrap(),
        None
    );
    parts
        .headers
        .insert("authorization", "valid".parse().unwrap());
    let user = User::from_request_parts(&mut parts, &state).await.unwrap();
    assert_eq!(Extract(user).into_inner(), User(42));
    parts
        .headers
        .insert("authorization", "revoked".parse().unwrap());
    assert_eq!(
        User::from_request_parts(&mut parts, &state)
            .await
            .unwrap_err()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(state.calls.load(Ordering::SeqCst), 3);
    assert_eq!(parts.extensions.get::<&str>(), Some(&"visited"));
}

#[cfg(feature = "http")]
mod adapter {
    use super::*;
    use simple_server::axum::{self, Router, body::Body, routing::post};
    use tower::ServiceExt;

    async fn required(Extract(user): Extract<User>, body: String) -> String {
        format!("{}:{body}", user.0)
    }

    async fn optional(Extract(user): Extract<Option<User>>, body: String) -> String {
        format!("{}:{body}", user.map_or(0, |u| u.0))
    }

    #[tokio::test]
    async fn handler_adapter_preserves_optional_policy_body_and_rejection_contract() {
        let state = State::default();
        let app = Router::new()
            .route("/required", post(required))
            .route("/optional", post(optional))
            .with_state(state.clone());
        for (path, credential, expected_status, expected_body) in [
            ("/required", None, StatusCode::UNAUTHORIZED, ""),
            ("/required", Some("invalid"), StatusCode::UNAUTHORIZED, ""),
            ("/required", Some("valid"), StatusCode::OK, "42:payload"),
            ("/optional", None, StatusCode::OK, "0:payload"),
            ("/optional", Some("invalid"), StatusCode::OK, "0:payload"),
            ("/optional", Some("valid"), StatusCode::OK, "42:payload"),
            (
                "/required",
                Some("busy"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database busy\0",
            ),
            (
                "/optional",
                Some("busy"),
                StatusCode::SERVICE_UNAVAILABLE,
                "database busy\0",
            ),
        ] {
            let mut request = http::Request::builder().method("POST").uri(path);
            if let Some(value) = credential {
                request = request.header("authorization", value);
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::from("payload")).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected_status);
            if expected_status == StatusCode::SERVICE_UNAVAILABLE {
                assert_eq!(response.headers()["retry-after"], "1");
                assert_eq!(response.headers().get_all("set-cookie").iter().count(), 2);
                assert_eq!(response.extensions().get::<usize>(), Some(&73));
            }
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), expected_body.as_bytes());
        }
        assert_eq!(state.calls.load(Ordering::SeqCst), 8);
    }

    #[tokio::test]
    async fn sequential_extractors_share_mutations_and_do_not_cache_values() {
        async fn twice(
            Extract(_): Extract<User>,
            Extract(_): Extract<User>,
            request: axum::extract::Request,
        ) -> String {
            assert_eq!(request.extensions().get::<&str>(), Some(&"visited"));
            let bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
                .await
                .unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        }
        let state = State::default();
        let app = Router::new()
            .route("/", post(twice))
            .with_state(state.clone());
        let response = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("authorization", "valid")
                    .body(Body::from("untouched"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
            "untouched"
        );
    }
}
