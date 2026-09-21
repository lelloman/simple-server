//! Optional HTTP request spans and response-body lifecycle events.
//!
//! [`trace`](crate::http_tracing::trace) wraps an Axum request callback. It records safe route templates,
//! status, header latency and body lifetime without buffering or reading bodies.
//! It does not install a subscriber. See `docs/step-03c-http-tracing.md` for
//! event semantics, placement and upgrade boundaries.

use axum::{
    body::{Body, Bytes, HttpBody},
    extract::MatchedPath,
    http::{Method, Request, StatusCode},
    response::Response,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tracing::{Instrument, Span};

/// Trace one request and its returned body, using the application's subscriber.
///
/// Install with `axum::middleware::from_fn(|request, next: axum::middleware::Next|
/// trace(request, |request| next.run(request)))`, or call inside an existing
/// middleware. Include response normalization inside the callback so the traced
/// body is the body actually returned. Correlation, if used, must surround this
/// call. The callback is constructed and polled inside the request span.
///
/// HTTP 101 and successful CONNECT responses finish at upgrade handoff; this
/// does not trace the resulting session. Body completion means frames consumed
/// by the server, not bytes acknowledged by the client. Dropping the future or
/// body records cancellation, which is not necessarily a client disconnect.
pub async fn trace<F, Fut>(request: Request<Body>, next: F) -> Response
where
    F: FnOnce(Request<Body>) -> Fut,
    Fut: Future<Output = Response>,
{
    trace_with_observer(request, TracingObserver, next).await
}

/// Trace with application-owned response and completion events.
///
/// Replaces automatic events, while retaining the same safe span, timing and
/// body lifecycle. The observer can write to an existing sink without installing
/// a tracing subscriber. Delegate to [`TracingObserver`] to retain selected
/// default events. The callback placement rules of [`trace`] still apply.
pub async fn trace_with_observer<F, Fut, O>(
    request: Request<Body>,
    observer: O,
    next: F,
) -> Response
where
    F: FnOnce(Request<Body>) -> Fut,
    Fut: Future<Output = Response>,
    O: Observer,
{
    let started = Instant::now();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .map(|path| {
            if path.len() <= 256 {
                path
            } else {
                "<route-too-long>"
            }
        })
        .unwrap_or("<unmatched>");
    let method = match *request.method() {
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::PATCH => "PATCH",
        Method::DELETE => "DELETE",
        Method::HEAD => "HEAD",
        Method::OPTIONS => "OPTIONS",
        Method::CONNECT => "CONNECT",
        Method::TRACE => "TRACE",
        _ => "OTHER",
    };
    let connect = request.method() == Method::CONNECT;
    let head = request.method() == Method::HEAD;
    let span = tracing::info_span!(
        "http.request",
        method,
        route,
        request_id = tracing::field::Empty,
        status = tracing::field::Empty
    );
    // Only a validated shared ID is safe for automatic recording. Opaque legacy
    // IDs remain application-owned and may be attached to a parent span.
    #[cfg(feature = "correlation")]
    if let Some(id) = crate::correlation::current_id() {
        span.record("request_id", id.as_str());
    }
    let mut observation = Observation {
        span: span.clone(),
        started,
        finished: false,
        headers: false,
        observer: Box::new(observer),
    };
    let response = async move { next(request).await }
        .instrument(span.clone())
        .await;
    let status = response.status();
    span.record("status", status.as_u16());
    observation.headers = true;
    {
        let _entered = span.enter();
        observation
            .observer
            .on_response(&span, &response, started.elapsed());
    }
    if status == StatusCode::SWITCHING_PROTOCOLS || (connect && status.is_success()) {
        observation.finish(Outcome::Upgraded);
        return response;
    }
    // The HTTP stack discards these bodies by protocol, not cancellation.
    // Leave the response intact so it can preserve HEAD content-length semantics.
    if head
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED
        || status.is_informational()
    {
        observation.finish(Outcome::Complete);
        return response;
    }
    let (parts, body) = response.into_parts();
    if body.is_end_stream() {
        observation.finish(Outcome::Complete);
        return Response::from_parts(parts, body);
    }
    Response::from_parts(
        parts,
        Body::new(ObservedBody {
            inner: Box::pin(body),
            observation,
        }),
    )
}

/// Where the observed lifecycle ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The callback had not returned a response.
    Headers,
    /// A response was returned, including protocol bodyless responses/upgrades.
    Body,
}
impl Phase {
    /// Stable field value used by default events.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Headers => "headers",
            Self::Body => "body",
        }
    }
}

/// Terminal HTTP body outcome, independent of response status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// End of body, or a response with no protocol wire body.
    Complete,
    /// HTTP 101 or successful CONNECT handoff, not session completion.
    Upgraded,
    /// Body polling returned an error.
    Error,
    /// Future/body dropped before its terminal outcome was observed.
    Cancelled,
}
impl Outcome {
    /// Stable field value used by default events.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Upgraded => "upgraded",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Application-owned event policy. Methods default to no output.
///
/// Called synchronously inside the shared request span. `on_response` runs once
/// after response creation, unless the future is cancelled before that point.
/// `on_finish` runs exactly once after an observed terminal outcome, including
/// from Drop. Callbacks must not panic or block waiting for asynchronous work;
/// they execute synchronously and should return promptly.
/// Observer state must outlive request handling because it may follow the body.
/// Response headers/extensions are read-only; privacy of custom output remains
/// the application's responsibility. No subscriber is required for callbacks.
pub trait Observer: Send + 'static {
    /// Response creation latency, measured before the observer executes.
    fn on_response(&mut self, _span: &Span, _response: &Response, _latency: Duration) {}
    /// Total elapsed time since tracing began (not client acknowledgment).
    fn on_finish(&mut self, _span: &Span, _outcome: Outcome, _phase: Phase, _duration: Duration) {}
}

/// The default event policy used by [`trace`].
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingObserver;
impl Observer for TracingObserver {
    fn on_response(&mut self, span: &Span, response: &Response, latency: Duration) {
        let status = response.status().as_u16();
        let header_latency_ms = latency.as_secs_f64() * 1000.0;
        if response.status().is_server_error() {
            tracing::error!(parent: span, status, header_latency_ms, "http.response_headers");
        } else {
            tracing::debug!(parent: span, status, header_latency_ms, "http.response_headers");
        }
    }
    fn on_finish(&mut self, span: &Span, outcome: Outcome, phase: Phase, duration: Duration) {
        let phase = phase.as_str();
        let outcome = outcome.as_str();
        let duration_ms = duration.as_secs_f64() * 1000.0;
        match outcome {
            "error" => tracing::error!(parent: span, outcome, phase, duration_ms, "http.finished"),
            "cancelled" => {
                tracing::warn!(parent: span, outcome, phase, duration_ms, "http.finished")
            }
            _ => tracing::info!(parent: span, outcome, phase, duration_ms, "http.finished"),
        }
    }
}

struct Observation {
    span: Span,
    started: Instant,
    finished: bool,
    headers: bool,
    observer: Box<dyn Observer>,
}
impl Observation {
    fn finish(&mut self, outcome: Outcome) {
        if self.finished {
            return;
        }
        self.finished = true;
        let phase = if self.headers {
            Phase::Body
        } else {
            Phase::Headers
        };
        let _entered = self.span.enter();
        self.observer
            .on_finish(&self.span, outcome, phase, self.started.elapsed());
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        self.finish(Outcome::Cancelled);
    }
}

struct ObservedBody {
    inner: Pin<Box<Body>>,
    observation: Observation,
}
impl HttpBody for ObservedBody {
    type Data = Bytes;
    type Error = axum::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
        let span = self.observation.span.clone();
        let _entered = span.enter();
        let result = self.inner.as_mut().poll_frame(cx);
        match &result {
            Poll::Ready(Some(Err(_))) => self.observation.finish(Outcome::Error),
            Poll::Ready(None) => self.observation.finish(Outcome::Complete),
            Poll::Ready(Some(Ok(_))) if self.inner.is_end_stream() => {
                self.observation.finish(Outcome::Complete)
            }
            _ => {}
        }
        result
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}
