//! Request identifiers shared by headers and application code.
//!
//! Wrap the complete router, including rejection paths. This module never
//! installs a logger, creates spans, or reads or modifies response bodies.
//! Task-local context lasts until response creation, not body streaming, and
//! is not inherited by spawned tasks. Capture
//! [`RequestId`](crate::correlation::RequestId) for deferred work.

use std::{fmt, future::Future};

use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

tokio::task_local! {
    static CURRENT: RequestId;
}

/// An identifier suitable for headers, logs and application error envelopes.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RequestId(String);

impl RequestId {
    /// Generate 128 random bits encoded as 32 lowercase hexadecimal characters.
    pub fn generate() -> Self {
        let bytes: [u8; 16] = rand::random();
        Self(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    /// Accept 1–64 ASCII letters, digits, hyphens, underscores, dots or colons.
    pub fn parse(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')))
        .then(|| Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The current request's ID, or `None` outside the request future's scope.
pub fn current_id() -> Option<RequestId> {
    CURRENT.try_with(Clone::clone).ok()
}

/// Caller ID trust is opt-in. IDs are correlation labels, never authorization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IncomingIds {
    #[default]
    Generate,
    /// Validate the first header value; ignore subsequent values. Invalid or
    /// missing values cause generation. Comma-separated lists are invalid.
    AcceptValidated,
}

/// Request ID selection and propagation. Default header: `x-request-id`.
#[derive(Clone, Debug)]
pub struct Correlation {
    header: HeaderName,
    incoming: IncomingIds,
}

impl Default for Correlation {
    fn default() -> Self {
        Self::new(HeaderName::from_static("x-request-id"))
    }
}

impl Correlation {
    pub fn new(header: HeaderName) -> Self {
        Self {
            header,
            incoming: IncomingIds::Generate,
        }
    }

    pub fn incoming_ids(mut self, incoming: IncomingIds) -> Self {
        self.incoming = incoming;
        self
    }

    /// Select one ID, set request extensions and scope the downstream future.
    /// Replace the response header and extension with the selected ID. Incoming
    /// request headers are unchanged. Bodies and status codes are untouched.
    pub async fn run<F, Fut>(&self, mut request: Request, next: F) -> Response
    where
        F: FnOnce(Request) -> Fut,
        Fut: Future<Output = Response>,
    {
        let id = if self.incoming == IncomingIds::AcceptValidated {
            request
                .headers()
                .get(&self.header)
                .and_then(|value| value.to_str().ok())
                .and_then(RequestId::parse)
        } else {
            None
        }
        .unwrap_or_else(RequestId::generate);
        request.extensions_mut().insert(id.clone());
        // Invoke the callback inside the scope as well as polling its future.
        let mut response = CURRENT
            .scope(id.clone(), async { next(request).await })
            .await;
        response.headers_mut().insert(
            self.header.clone(),
            HeaderValue::from_str(id.as_str()).expect("validated request ID"),
        );
        response.extensions_mut().insert(id);
        response
    }
}

/// Adapter for `axum::middleware::from_fn_with_state(config, middleware)`.
pub async fn middleware(
    State(config): State<Correlation>,
    request: Request,
    next: Next,
) -> Response {
    config.run(request, |request| next.run(request)).await
}
