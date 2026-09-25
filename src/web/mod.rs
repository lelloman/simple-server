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

mod body;
mod extract;
mod handler;
mod response;
pub mod routing;

pub use crate::extract::{Extract, FromRequestParts, IntoRejectionResponse, RejectionResponse};
pub use body::{Body, BodyError};
pub use bytes::Bytes;
pub use extract::{FromRequest, FromState, Json, Path, Query, State};
pub use handler::Handler;
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

/// Temporary, explicitly enabled migration boundary for legacy applications.
/// Remove these calls when the parent application uses shared routing as well.
#[cfg(feature = "web-compat")]
pub mod compat {
    use super::Router;

    /// Compose a migrated route group into an existing backend router. This is
    /// the only intentional backend type escape in the shared web API.
    pub fn into_axum_router<S>(router: Router<S>) -> axum::Router<S> {
        router.inner
    }
}
