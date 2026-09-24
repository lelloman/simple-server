use super::{Admission, AdmissionFuture, AsyncPolicy, Reason, Rejection};
use http::{Request, Response, StatusCode, request::Parts};
use http_body::{Body, Frame, SizeHint};
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tower_layer::Layer;
use tower_service::Service;

/// Default rejection renderer; custom renderers can preserve application errors.
/// Unknown retry times omit Retry-After. Capacity and clock exhaustion use 503;
/// an impossible cost uses 422; request rate/cooldown/concurrency use 429.
pub fn rejection_response<B: From<&'static str>>(error: Rejection) -> Response<B> {
    let mut response = Response::new(B::from("Request admission rejected"));
    *response.status_mut() = match error.reason {
        Reason::StoreCapacity | Reason::ClockRange => StatusCode::SERVICE_UNAVAILABLE,
        Reason::CostExceedsCapacity => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::TOO_MANY_REQUESTS,
    };
    if let Some(seconds) = error.retry_after_seconds() {
        response
            .headers_mut()
            .insert(http::header::RETRY_AFTER, seconds.into());
    }
    response
}

pin_project! {
    /// Streams the original body unchanged while holding concurrency guards until
    /// EOF, body error or drop. Rate charges are never refunded. WebSocket guards
    /// cover the HTTP upgrade only; hold a separate admission for a socket lifetime.
    pub struct RateLimitBody<B> {
        #[pin]
        inner: B,
        admission: Option<Admission>,
    }
}
impl<B: Body> RateLimitBody<B> {
    fn new(inner: B, admission: Admission) -> Self {
        let admission = if inner.is_end_stream() {
            None
        } else {
            Some(admission)
        };
        Self { inner, admission }
    }
}
impl<B: Body> Body for RateLimitBody<B> {
    type Data = B::Data;
    type Error = B::Error;
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();
        let frame = this.inner.as_mut().poll_frame(cx);
        if matches!(frame, Poll::Ready(None | Some(Err(_)))) || this.inner.is_end_stream() {
            this.admission.take();
        }
        frame
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

/// HTTP admission using application callbacks on standard request Parts. No body
/// reading, proxy trust, exemptions, retries, queue or background tasks. The error
/// renderer sees the original parts for logging, route labels and wire responses.
pub struct RateLimitLayer<E, R> {
    policy: AsyncPolicy<Parts, E>,
    reject: R,
}
impl<E, R: Clone> Clone for RateLimitLayer<E, R> {
    fn clone(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            reject: self.reject.clone(),
        }
    }
}
impl<E, R> RateLimitLayer<E, R> {
    pub fn new(policy: AsyncPolicy<Parts, E>, reject: R) -> Self {
        Self { policy, reject }
    }
}
impl<S, E, R: Clone> Layer<S> for RateLimitLayer<E, R> {
    type Service = RateLimitService<S, E, R>;
    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            policy: self.policy.clone(),
            reject: self.reject.clone(),
        }
    }
}
pub struct RateLimitService<S, E, R> {
    inner: S,
    policy: AsyncPolicy<Parts, E>,
    reject: R,
}
impl<S: Clone, E, R: Clone> Clone for RateLimitService<S, E, R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            policy: self.policy.clone(),
            reject: self.reject.clone(),
        }
    }
}
impl<S, E, R, B, Out> Service<Request<B>> for RateLimitService<S, E, R>
where
    S: Service<Request<B>, Response = Response<Out>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Send,
    E: Send + 'static,
    R: Fn(E, &Parts) -> Response<Out> + Clone + Send + 'static,
    B: Send + 'static,
    Out: Body + 'static,
{
    type Response = Response<RateLimitBody<Out>>;
    type Error = S::Error;
    type Future = AdmissionFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: Request<B>) -> Self::Future {
        let replacement = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, replacement);
        let policy = self.policy.clone();
        let reject = self.reject.clone();
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            match policy.evaluate(&parts).await {
                Ok(admission) => {
                    let response = inner.call(Request::from_parts(parts, body)).await?;
                    Ok(response.map(|body| RateLimitBody::new(body, admission)))
                }
                Err(error) => Ok(reject(error, &parts)
                    .map(|body| RateLimitBody::new(body, Admission::unrestricted()))),
            }
        })
    }
}
