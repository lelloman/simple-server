//! Backend-independent HTTP tracing with application-owned event policy.
//!
//! Requires `web` and `http-tracing`; no compatibility feature is needed.
//! Uses the same safe route spans, timing and lazy body lifecycle as the legacy
//! tracing API. Correlation middleware must surround tracing, and response
//! normalization must run inside its callback. No subscriber is installed.

use super::{Body, Request, Response};
use std::{future::Future, time::Duration};
use tracing::Span;

pub use crate::http_tracing::{Outcome, Phase};

/// Borrowed, read-only response metadata. The body is deliberately inaccessible.
///
/// Extensions remain available for application error codes and metrics. Custom
/// observers are responsible for redacting sensitive headers and extensions.
#[derive(Clone, Copy, Debug)]
pub struct ResponseInfo<'a> {
    status: http::StatusCode,
    version: http::Version,
    headers: &'a http::HeaderMap,
    extensions: &'a http::Extensions,
}

impl<'a> ResponseInfo<'a> {
    /// Inspect any HTTP response without depending on its body representation.
    pub fn new<B>(response: &'a http::Response<B>) -> Self {
        Self {
            status: response.status(),
            version: response.version(),
            headers: response.headers(),
            extensions: response.extensions(),
        }
    }

    pub fn status(&self) -> http::StatusCode {
        self.status
    }
    pub fn version(&self) -> http::Version {
        self.version
    }
    pub fn headers(&self) -> &'a http::HeaderMap {
        self.headers
    }
    pub fn extensions(&self) -> &'a http::Extensions {
        self.extensions
    }
}

/// Synchronous callbacks inside the request span; defaults emit nothing.
///
/// `on_response` runs once after headers are created. `on_finish` runs once on
/// completion, error, upgrade handoff or cancellation (including Drop), provided
/// the tracing future was polled. State can outlive the handler while its body
/// streams. Callbacks must not panic or block. No subscriber is required.
pub trait Observer: Send + 'static {
    fn on_response(&mut self, _span: &Span, _response: &ResponseInfo<'_>, _latency: Duration) {}
    fn on_finish(&mut self, _span: &Span, _outcome: Outcome, _phase: Phase, _duration: Duration) {}
}

/// Default events, identical to the legacy tracing policy.
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingObserver;

impl Observer for TracingObserver {
    fn on_response(&mut self, span: &Span, response: &ResponseInfo<'_>, latency: Duration) {
        crate::http_tracing::response_event(span, response.status(), latency);
    }
    fn on_finish(&mut self, span: &Span, outcome: Outcome, phase: Phase, duration: Duration) {
        crate::http_tracing::Observer::on_finish(
            &mut crate::http_tracing::TracingObserver,
            span,
            outcome,
            phase,
            duration,
        );
    }
}

/// Trace an owned request/response with the default event policy.
///
/// Body frames are never buffered or eagerly consumed. Completion measures
/// server consumption, not client acknowledgment; upgrades end at handoff.
/// Dropping the future/body reports cancellation, not necessarily disconnect.
pub async fn trace<F, Fut>(request: Request, next: F) -> Response
where
    F: FnOnce(Request) -> Fut,
    Fut: Future<Output = Response>,
{
    trace_with_observer(request, TracingObserver, next).await
}

/// Replace default events with custom callbacks, retaining span/body semantics.
///
/// Delegate selected callbacks to `TracingObserver` to retain default events.
pub async fn trace_with_observer<F, Fut, O>(request: Request, observer: O, next: F) -> Response
where
    F: FnOnce(Request) -> Fut,
    Fut: Future<Output = Response>,
    O: Observer,
{
    crate::http_tracing::trace_with_observer(
        request.map(|body| body.0),
        Adapter(observer),
        |request| async move {
            next(super::service::request(request))
                .await
                .map(|body| body.0)
        },
    )
    .await
    .map(Body)
}

struct Adapter<O>(O);
impl<O: Observer> crate::http_tracing::Observer for Adapter<O> {
    fn on_response(&mut self, span: &Span, response: &axum::response::Response, latency: Duration) {
        self.0
            .on_response(span, &ResponseInfo::new(response), latency);
    }
    fn on_finish(&mut self, span: &Span, outcome: Outcome, phase: Phase, duration: Duration) {
        self.0.on_finish(span, outcome, phase, duration);
    }
}
