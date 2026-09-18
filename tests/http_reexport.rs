#![cfg(feature = "http")]

use simple_server::axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Path, State},
    http::{Request, StatusCode},
    routing::get,
};
use tower::ServiceExt;

#[tokio::test]
async fn consumer_router_preserves_state_path_and_method_handling() {
    let app = Router::new()
        .route(
            "/items/{id}",
            get(
                |State(prefix): State<String>, Path(id): Path<String>| async move {
                    format!("{prefix}:{id}")
                },
            ),
        )
        .with_state("item".to_owned());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/items/42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap(),
        "item:42"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/items/42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
