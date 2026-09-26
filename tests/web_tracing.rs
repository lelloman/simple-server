#![cfg(all(feature = "http-tracing", feature = "web"))]
use futures_util::{FutureExt, StreamExt, stream};
use http_body_util::{BodyExt, StreamBody};
use simple_server::{
    web::tracing::{Observer, Outcome, Phase, trace, trace_with_observer},
    web::{
        Router,
        body::{Body, Bytes, to_bytes},
        http::{HeaderMap, Request, Response, StatusCode},
        middleware::{self, Next},
        routing::get,
    },
};
type ResponseView<'a> = &'a simple_server::web::tracing::ResponseInfo<'a>;
include!("http_tracing_contract/mod.rs");

#[test]
fn metadata_view_accepts_unrelated_body_types_and_preserves_version() {
    use simple_server::web::{http::Version, tracing::ResponseInfo};
    let mut response = Response::builder()
        .status(202)
        .version(Version::HTTP_2)
        .header("x-app", "kept")
        .body(vec![1_u8, 2, 3])
        .unwrap();
    response.extensions_mut().insert(42_u32);
    let info = ResponseInfo::new(&response);
    assert_eq!(info.status(), StatusCode::ACCEPTED);
    assert_eq!(info.version(), Version::HTTP_2);
    assert_eq!(info.headers()["x-app"], "kept");
    assert_eq!(info.extensions().get::<u32>(), Some(&42));
    assert!(std::ptr::eq(info.headers(), response.headers()));
    assert!(std::ptr::eq(info.extensions(), response.extensions()));
    assert_eq!(response.body(), &[1, 2, 3]);
}
