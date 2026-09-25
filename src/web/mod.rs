//! Framework-independent routing, handlers, extractors and responses.
//!
//! These are simple-server's own contracts, not aliases for backend types.
//! See `docs/web-core.md` for supported behavior and migration boundaries.
//!
//! Body-consuming arguments must come last:
//! ```compile_fail
//! use simple_server::web::{Json, State, Router, routing::post};
//! async fn invalid(Json(_): Json<String>, State(_): State<()>) {}
//! let _: Router = Router::new().route("/", post(invalid));
//! ```
//! Two body-consuming arguments are not permitted:
//! ```compile_fail
//! use simple_server::web::{Json, Router, routing::post};
//! async fn invalid(Json(_): Json<String>, _: String) {}
//! let _: Router = Router::new().route("/", post(invalid));
//! ```

pub mod body;
pub mod extract;
mod handler;
pub mod middleware;
pub mod response;
pub mod routing;
mod service;

pub use crate::extract::{Extract, FromRequestParts, IntoRejectionResponse, RejectionResponse};
pub use body::{Body, BodyError};
pub use bytes::Bytes;
pub use extract::{
    ConnectInfo, Extension, FromRequest, FromState, Json, MatchedPath, Path, Query, State,
};
pub use handler::Handler;
pub use http;
pub use http::{Extensions, HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
pub use response::IntoResponse;
pub use routing::Router;

pub type Request<B = Body> = http::Request<B>;
pub type Response<B = Body> = http::Response<B>;

/// Serve a fully configured shared router with explicit lifecycle shutdown.
#[cfg(feature = "lifecycle")]
pub async fn serve(
    listener: tokio::net::TcpListener,
    router: Router,
    shutdown: crate::lifecycle::Shutdown,
) -> std::io::Result<()> {
    crate::http::serve(listener, router.inner, shutdown).await
}

/// Serve with the direct TCP peer address in `ConnectInfo<SocketAddr>`.
/// Forwarded headers are deliberately not interpreted.
#[cfg(feature = "lifecycle")]
pub async fn serve_with_connect_info(
    listener: tokio::net::TcpListener,
    router: Router,
    shutdown: crate::lifecycle::Shutdown,
) -> std::io::Result<()> {
    crate::http::serve(
        listener,
        router
            .inner
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
        shutdown,
    )
    .await
}

/// Explicit migration boundaries for protocols not yet owned by the shared API.
#[cfg(feature = "web-compat")]
pub mod compat;
