//! Host-side middleware for services registered with engine routes.
//!
//! Layers wrap the supplied service, not the native router. Engine-generated
//! 404/405 responses and sibling routes are not intercepted. Host extensions,
//! streaming bodies and readiness stay within one callback; no native ABI change
//! is required. Router-wide layers and routing continuations remain separate work.
use super::Service;
use super::{FromRequestParts, IntoRejectionResponse, IntoResponse, Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
/// The runtime-independent Tower layer contract.
pub use tower_layer::Layer;

type ResponseFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

/// An owned host continuation. Each run clones its inner service, then waits
/// for readiness on that exact clone. Clones can be used for application-owned
/// retry/fan-out policy; requests and bodies themselves are never cloned here.
#[derive(Clone)]
pub struct Next(Arc<dyn Fn(Request) -> ResponseFuture + Send + Sync>);
impl Next {
    fn new<I>(inner: I) -> Self
    where
        I: Service<Request, Response = Response, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        I::Future: Send + 'static,
    {
        Self(Arc::new(move |request| {
            Box::pin(super::service::call(inner.clone(), request))
        }))
    }
    pub async fn run(self, request: Request) -> Response {
        (self.0)(request).await
    }
}
impl Service<Request> for Next {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: Request) -> Self::Future {
        let future = (self.0)(request);
        Box::pin(async move { Ok(future.await) })
    }
}

/// Bind a typed handler and its state into a host service for middleware layers.
/// State is explicit here, independently of the enclosing router's missing state.
///
/// ```no_run
/// use simple_server::engine_web::{Method, MethodRouter, Router, State};
/// use simple_server::engine_web::middleware::{handler_service, map_response, Layer};
/// async fn example() -> std::io::Result<Router> {
///     let handler = handler_service(|State(value): State<String>| async move { value },
///                                  "hello".to_owned());
///     let service = map_response(|mut response: simple_server::engine_web::Response| async move {
///         response.headers_mut().insert("x-app", "example".parse().unwrap());
///         response
///     }).layer(handler);
///     Router::new()?.route("/", MethodRouter::new()?.on_service(Method::GET, service)?)
/// }
/// ```
pub fn handler_service<H, T, S>(handler: H, state: S) -> Next
where
    H: super::Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    Next(Arc::new(move |request| {
        Box::pin(handler.clone().call(request, state.clone()))
    }))
}

/// Implemented for async functions taking head extractors, a request and `Next`.
pub trait Middleware<T, S>: Clone + Send + Sync + 'static {
    fn call(self, request: Request, state: S, next: Next) -> ResponseFuture;
}

pub struct FromFnLayer<F, S, T> {
    function: F,
    state: S,
    marker: PhantomData<fn() -> T>,
}
impl<F: Clone, S: Clone, T> Clone for FromFnLayer<F, S, T> {
    fn clone(&self) -> Self {
        Self {
            function: self.function.clone(),
            state: self.state.clone(),
            marker: PhantomData,
        }
    }
}
pub fn from_fn<F, T>(function: F) -> FromFnLayer<F, (), T>
where
    F: Middleware<T, ()>,
{
    from_fn_with_state((), function)
}
pub fn from_fn_with_state<F, S, T>(state: S, function: F) -> FromFnLayer<F, S, T>
where
    F: Middleware<T, S>,
{
    FromFnLayer {
        function,
        state,
        marker: PhantomData,
    }
}
impl<F, S, T, I> tower_layer::Layer<I> for FromFnLayer<F, S, T>
where
    F: Clone,
    S: Clone,
{
    type Service = FromFn<F, S, T, I>;
    fn layer(&self, inner: I) -> Self::Service {
        FromFn {
            layer: self.clone(),
            inner,
        }
    }
}
pub struct FromFn<F, S, T, I> {
    layer: FromFnLayer<F, S, T>,
    inner: I,
}
impl<F: Clone, S: Clone, T, I: Clone> Clone for FromFn<F, S, T, I> {
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
            inner: self.inner.clone(),
        }
    }
}
impl<F, S, T, I> Service<Request> for FromFn<F, S, T, I>
where
    F: Middleware<T, S>,
    S: Clone + Send + Sync + 'static,
    I: Service<Request, Response = Response, Error = Infallible> + Clone + Send + Sync + 'static,
    I::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;
    // `Next` waits for readiness on the cloned inner service, just before use.
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: Request) -> Self::Future {
        let next = Next::new(self.inner.clone());
        let future = self
            .layer
            .function
            .clone()
            .call(request, self.layer.state.clone(), next);
        Box::pin(async move { Ok(future.await) })
    }
}
macro_rules! middleware {
    ($($ty:ident),*) => {
        impl<F,Fut,R,S,$($ty,)*> Middleware<($($ty,)*),S> for F
        where F: FnOnce($($ty,)* Request,Next)->Fut + Clone + Send + Sync + 'static,
              Fut: Future<Output=R> + Send + 'static, R: IntoResponse,
              S: Send + Sync + 'static, $($ty: FromRequestParts<S> + Send,)*
        {
            #[allow(non_snake_case, unused_mut, unused_variables)]
            fn call(self, request: Request, state: S, next: Next) -> ResponseFuture {
                Box::pin(async move {
                    let (mut parts,body) = request.into_parts();
                    $(let $ty = match $ty::from_request_parts(&mut parts,&state).await {
                        Ok(value) => value,
                        Err(error) => return error.into_rejection_response().into_response(),
                    };)*
                    self($($ty,)* Request::from_parts(parts,body),next).await.into_response()
                })
            }
        }
    };
}
middleware!();
middleware!(A);
middleware!(A, B);
middleware!(A, B, C);
middleware!(A, B, C, D);
middleware!(A, B, C, D, E);
middleware!(A, B, C, D, E, F1);
middleware!(A, B, C, D, E, F1, G);
middleware!(A, B, C, D, E, F1, G, H);

/// Map downstream responses, including handler extraction failures.
pub fn map_response<F, Fut, R>(function: F) -> FromFnLayer<ResponseMapper<F>, (), ()>
where
    F: FnOnce(Response) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    from_fn(ResponseMapper(function))
}
#[derive(Clone)]
pub struct ResponseMapper<F>(F);
impl<F, Fut, R> Middleware<(), ()> for ResponseMapper<F>
where
    F: FnOnce(Response) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    fn call(self, request: Request, _: (), next: Next) -> ResponseFuture {
        Box::pin(async move { (self.0)(next.run(request).await).await.into_response() })
    }
}
