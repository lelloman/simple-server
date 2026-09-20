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
    static CURRENT: Context;
}

#[derive(Clone)]
struct Context {
    validated: Option<RequestId>,
    header: HeaderRequestId,
}

/// An application-selected ID. Unlike [`RequestId`], this preserves any legal
/// HTTP header bytes. The application owns validation, trust and generation.
/// Do not assume it is UTF-8, bounded to 64 bytes or safe to interpolate in logs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeaderRequestId(HeaderValue);

impl HeaderRequestId {
    pub fn new(value: HeaderValue) -> Self {
        Self(value)
    }

    pub fn as_header_value(&self) -> &HeaderValue {
        &self.0
    }
}

/// Current selected header ID, including IDs from the default validated path.
pub fn current_header_id() -> Option<HeaderRequestId> {
    CURRENT.try_with(|context| context.header.clone()).ok()
}

/// Whether an application-selected ID replaces an existing response header.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResponseHeader {
    #[default]
    Overwrite,
    /// Keep a downstream header when present. Response extensions reflect that
    /// final header; the request scope retains the original selected ID.
    Preserve,
}

/// Explicit compatibility policy for application-selected identifiers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RequestHeader {
    #[default]
    Unchanged,
    /// Set only when no header is present; retain repeated values.
    IfMissing,
    Overwrite,
}

/// Header propagation for application-selected identifiers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Propagation {
    pub request_header: RequestHeader,
    pub response_header: ResponseHeader,
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
    CURRENT
        .try_with(|context| context.validated.clone())
        .ok()
        .flatten()
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
    pub async fn run<F, Fut>(&self, request: Request, next: F) -> Response
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
        let header =
            HeaderRequestId::new(HeaderValue::from_str(id.as_str()).expect("validated request ID"));
        self.run_inner(
            request,
            Context {
                validated: Some(id),
                header,
            },
            Propagation::default(),
            next,
        )
        .await
    }

    /// Run with an ID selected and validated by the application, bypassing
    /// `IncomingIds` and the default generator. This supports legacy wire
    /// contracts, strict rejection policies and custom ID formats.
    ///
    /// Sets [`HeaderRequestId`] extensions and [`current_header_id`]. It clears
    /// the validated [`RequestId`] extension and makes [`current_id`] return
    /// `None` within this scope: opaque values must not masquerade as validated
    /// IDs. The callback may add the application's own extension or span.
    pub async fn run_selected<F, Fut>(
        &self,
        request: Request,
        id: HeaderRequestId,
        propagation: Propagation,
        next: F,
    ) -> Response
    where
        F: FnOnce(Request) -> Fut,
        Fut: Future<Output = Response>,
    {
        self.run_inner(
            request,
            Context {
                validated: None,
                header: id,
            },
            propagation,
            next,
        )
        .await
    }

    async fn run_inner<F, Fut>(
        &self,
        mut request: Request,
        context: Context,
        propagation: Propagation,
        next: F,
    ) -> Response
    where
        F: FnOnce(Request) -> Fut,
        Fut: Future<Output = Response>,
    {
        request.extensions_mut().remove::<RequestId>();
        if let Some(id) = &context.validated {
            request.extensions_mut().insert(id.clone());
        }
        request.extensions_mut().insert(context.header.clone());
        if propagation.request_header == RequestHeader::Overwrite
            || (propagation.request_header == RequestHeader::IfMissing
                && !request.headers().contains_key(&self.header))
        {
            request
                .headers_mut()
                .insert(self.header.clone(), context.header.0.clone());
        }
        // Invoke the callback inside the scope as well as polling its future.
        let mut response = CURRENT
            .scope(context.clone(), async { next(request).await })
            .await;
        let final_id = match (
            propagation.response_header,
            response.headers().get(&self.header),
        ) {
            (ResponseHeader::Preserve, Some(value)) => HeaderRequestId::new(value.clone()),
            _ => {
                response
                    .headers_mut()
                    .insert(self.header.clone(), context.header.0.clone());
                context.header
            }
        };
        response.extensions_mut().remove::<RequestId>();
        if let Some(id) = context.validated {
            response.extensions_mut().insert(id);
        }
        response.extensions_mut().insert(final_id);
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
