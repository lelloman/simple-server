//! Opt-in extractor body limits, independent of lifecycle and logging.
//!
//! This preserves Axum's `DefaultBodyLimit` semantics. It limits cooperating
//! extractors (including bytes, JSON and multipart), not arbitrary reads of a
//! raw request body. It neither pre-buffers requests nor limits response bodies.

use axum::extract::DefaultBodyLimit;
use tower_layer::Layer;

/// A route-scoped limit in bytes for extractors that honor the default limit.
///
/// Applications own the byte count, placement and rejection handling. Existing
/// extractor status codes and response bodies are preserved. Multipart part/file
/// limits remain application-owned. Installing nothing keeps framework defaults.
///
/// ```
/// use simple_server::{axum::{Router, routing::post}, body_limit::BodyLimit};
/// let app: Router = Router::new()
///     .route("/upload", post(|body: String| async move { body }))
///     .layer(BodyLimit::max(1024));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BodyLimit {
    inner: DefaultBodyLimit,
}

impl BodyLimit {
    /// Set an explicit maximum. Zero permits only empty extracted bodies.
    pub const fn max(bytes: usize) -> Self {
        Self {
            inner: DefaultBodyLimit::max(bytes),
        }
    }
}

impl<S> Layer<S> for BodyLimit {
    type Service = <DefaultBodyLimit as Layer<S>>::Service;

    fn layer(&self, inner: S) -> Self::Service {
        self.inner.layer(inner)
    }
}
