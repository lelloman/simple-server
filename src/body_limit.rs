//! Opt-in extractor body limits, independent of lifecycle and logging.
//!
//! This preserves Axum's `DefaultBodyLimit` semantics. It limits cooperating
//! extractors (including bytes, JSON and multipart), not arbitrary reads of a
//! raw request body. It neither pre-buffers requests nor limits response bodies.

use axum::extract::DefaultBodyLimit;
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

/// A route-scoped limit in bytes for extractors that honor the default limit.
///
/// Applications own the byte count, placement and rejection handling. Existing
/// extractor status codes and response bodies are preserved. Multipart part/file
/// limits remain application-owned. Installing nothing keeps framework defaults.
///
/// ```
/// use simple_server::{web::{Router, routing::post}, body_limit::BodyLimit};
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
    type Service = BodyLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BodyLimitService(self.inner.layer(inner))
    }
}

/// Service with a route-scoped extractor limit. Its transport adapter is private.
#[derive(Clone, Debug)]
pub struct BodyLimitService<S>(<DefaultBodyLimit as Layer<S>>::Service);

impl<S, R> Service<R> for BodyLimitService<S>
where
    S: Service<R>,
    <DefaultBodyLimit as Layer<S>>::Service:
        Service<R, Response = S::Response, Error = S::Error, Future = S::Future>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, request: R) -> Self::Future {
        self.0.call(request)
    }
}
