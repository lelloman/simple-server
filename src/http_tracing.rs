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
    time::Instant,
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
    };
    let response = async move { next(request).await }
        .instrument(span.clone())
        .await;
    let status = response.status();
    span.record("status", status.as_u16());
    observation.headers = true;
    if status.is_server_error() {
        tracing::error!(parent: &span, status = status.as_u16(), header_latency_ms = elapsed_ms(started), "http.response_headers");
    } else {
        tracing::debug!(parent: &span, status = status.as_u16(), header_latency_ms = elapsed_ms(started), "http.response_headers");
    }
    if status == StatusCode::SWITCHING_PROTOCOLS || (connect && status.is_success()) {
        observation.finish("upgraded");
        return response;
    }
    // The HTTP stack discards these bodies by protocol, not cancellation.
    // Leave the response intact so it can preserve HEAD content-length semantics.
    if head
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED
        || status.is_informational()
    {
        observation.finish("complete");
        return response;
    }
    let (parts, body) = response.into_parts();
    if body.is_end_stream() {
        observation.finish("complete");
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

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

struct Observation {
    span: Span,
    started: Instant,
    finished: bool,
    headers: bool,
}
impl Observation {
    fn finish(&mut self, outcome: &'static str) {
        if self.finished {
            return;
        }
        self.finished = true;
        let phase = if self.headers { "body" } else { "headers" };
        if outcome == "error" {
            tracing::error!(parent: &self.span, outcome, phase, duration_ms = elapsed_ms(self.started), "http.finished");
        } else if outcome == "cancelled" {
            tracing::warn!(parent: &self.span, outcome, phase, duration_ms = elapsed_ms(self.started), "http.finished");
        } else {
            tracing::info!(parent: &self.span, outcome, phase, duration_ms = elapsed_ms(self.started), "http.finished");
        }
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        self.finish("cancelled");
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
            Poll::Ready(Some(Err(_))) => self.observation.finish("error"),
            Poll::Ready(None) => self.observation.finish("complete"),
            Poll::Ready(Some(Ok(_))) if self.inner.is_end_stream() => {
                self.observation.finish("complete")
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
