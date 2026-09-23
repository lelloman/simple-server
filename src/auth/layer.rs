use super::{AsyncAccess, AuthFuture};
use http::{Request, Response, request::Parts};
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

/// Identity established by this layer after all access checks succeed. Public
/// construction is intentionally absent. This is a request-local result, not a
/// durable authorization grant; resource-specific checks may still be required.
#[derive(Clone)]
pub struct Identity<P>(P);
impl<P> Identity<P> {
    pub fn principal(&self) -> &P {
        &self.0
    }
    pub fn into_principal(self) -> P {
        self.0
    }
}

/// Framework-independent HTTP gate using `http` and Tower. Mount explicitly on
/// protected routes. Public routes should be mounted outside this layer. It never
/// reads or buffers the body, logs credentials, redirects, or chooses status codes.
/// Rendering failures, including provider outages, is application-owned.
/// A pre-existing `Identity<P>` is removed before verification to prevent stale
/// identity reuse when stacking gates. Other request extensions are preserved.
pub struct AuthLayer<P, E, R> {
    access: AsyncAccess<Parts, P, E>,
    reject: R,
}
impl<P, E, R: Clone> Clone for AuthLayer<P, E, R> {
    fn clone(&self) -> Self {
        Self {
            access: self.access.clone(),
            reject: self.reject.clone(),
        }
    }
}
impl<P, E, R> AuthLayer<P, E, R> {
    pub fn new(access: AsyncAccess<Parts, P, E>, reject: R) -> Self {
        Self { access, reject }
    }
}
impl<S, P, E, R: Clone> Layer<S> for AuthLayer<P, E, R> {
    type Service = AuthService<S, P, E, R>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            access: self.access.clone(),
            reject: self.reject.clone(),
        }
    }
}
pub struct AuthService<S, P, E, R> {
    inner: S,
    access: AsyncAccess<Parts, P, E>,
    reject: R,
}
impl<S: Clone, P, E, R: Clone> Clone for AuthService<S, P, E, R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            access: self.access.clone(),
            reject: self.reject.clone(),
        }
    }
}
impl<S, P, E, R, B, Out> Service<Request<B>> for AuthService<S, P, E, R>
where
    S: Service<Request<B>, Response = Response<Out>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Send,
    P: Clone + Send + Sync + 'static,
    E: Send + 'static,
    R: Fn(E) -> Response<Out> + Clone + Send + 'static,
    B: Send + 'static,
    Out: 'static,
{
    type Response = Response<Out>;
    type Error = S::Error;
    type Future = AuthFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: Request<B>) -> Self::Future {
        // Move the instance whose readiness was polled, never call its unready clone.
        let replacement = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, replacement);
        let access = self.access.clone();
        let reject = self.reject.clone();
        Box::pin(async move {
            let (mut parts, body) = request.into_parts();
            parts.extensions.remove::<Identity<P>>();
            match access.evaluate(&parts).await {
                Ok(principal) => {
                    parts.extensions.insert(Identity(principal));
                    inner.call(Request::from_parts(parts, body)).await
                }
                Err(error) => Ok(reject(error)),
            }
        })
    }
}
