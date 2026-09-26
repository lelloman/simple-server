use super::{Body, Handler, Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_service::Service;

/// Route collection whose missing application state is `S`.
#[must_use]
#[derive(Clone, Debug)]
pub struct Router<S = ()> {
    pub(super) inner: axum::Router<S>,
}

impl<S: Clone + Send + Sync + 'static> Default for Router<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Clone + Send + Sync + 'static> Router<S> {
    pub fn new() -> Self {
        Self {
            inner: axum::Router::new(),
        }
    }

    pub fn route(mut self, path: &str, methods: MethodRouter<S>) -> Self {
        self.inner = self.inner.route(path, methods.inner);
        self
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.inner = self.inner.merge(other.inner);
        self
    }

    pub fn nest(mut self, path: &str, other: Self) -> Self {
        self.inner = self.inner.nest(path, other.inner);
        self
    }

    pub fn fallback<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.inner = self.inner.fallback(Adapter(handler));
        self
    }

    /// Apply a Tower layer to existing routes and fallbacks.
    pub fn layer<L, B>(mut self, layer: L) -> Self
    where
        L: tower_layer::Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request, Response = http::Response<B>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.inner = self.inner.layer(super::service::BackendLayer(layer));
        self
    }

    /// Apply a layer only after a route matches (does not intercept a 404).
    pub fn route_layer<L, B>(mut self, layer: L) -> Self
    where
        L: tower_layer::Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request, Response = http::Response<B>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.inner = self.inner.route_layer(super::service::BackendLayer(layer));
        self
    }

    pub fn fallback_service<T, B>(mut self, service: T) -> Self
    where
        T: Service<Request, Response = http::Response<B>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Future: Send + 'static,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.inner = self
            .inner
            .fallback_service(super::service::BackendService(service));
        self
    }

    /// Mount a standard Tower service below a prefix, stripping that prefix.
    pub fn nest_service<T, B>(mut self, path: &str, service: T) -> Self
    where
        T: Service<Request, Response = http::Response<B>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Future: Send + 'static,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.inner = self
            .inner
            .nest_service(path, super::service::BackendService(service));
        self
    }

    /// Fallback constrained by HTTP method, preserving HEAD/405/Allow behavior.
    pub fn fallback_methods(mut self, methods: MethodRouter<S>) -> Self {
        self.inner = self.inner.fallback(methods.inner);
        self
    }

    pub fn with_state<S2: Clone + Send + Sync + 'static>(self, state: S) -> Router<S2> {
        Router {
            inner: self.inner.with_state(state),
        }
    }

    /// Apply the existing shared extractor limit to routes already registered.
    /// This does not bound arbitrary raw-body reads; use `Body::collect` for those.
    #[cfg(feature = "body-limit")]
    pub fn body_limit(mut self, limit: crate::body_limit::BodyLimit) -> Self {
        self.inner = self.inner.layer(limit);
        self
    }
}

/// Method-specific handlers with framework-independent handler bounds.
#[derive(Clone, Debug)]
pub struct MethodRouter<S = ()> {
    inner: axum::routing::MethodRouter<S>,
}

impl<S: Clone + Send + Sync + 'static> Default for MethodRouter<S> {
    fn default() -> Self {
        Self {
            inner: axum::routing::MethodRouter::new(),
        }
    }
}

#[derive(Clone)]
struct Adapter<H>(H);
enum AdapterMarker {}

impl<H, T, S> axum::handler::Handler<(AdapterMarker, T), S> for Adapter<H>
where
    H: Handler<T, S>,
    T: 'static,
    S: 'static,
{
    type Future = Pin<Box<dyn Future<Output = axum::response::Response> + Send>>;
    fn call(self, request: axum::extract::Request, state: S) -> Self::Future {
        let future = self.0.call(super::service::request(request), state);
        Box::pin(async move { future.await.map(|body| body.0) })
    }
}

macro_rules! methods {
    ($($method:ident),+ $(,)?) => {$(
        pub fn $method<H, T, S>(handler: H) -> MethodRouter<S>
        where H: Handler<T, S>, T: 'static, S: Clone + Send + Sync + 'static,
        {
            MethodRouter { inner: axum::routing::$method(Adapter(handler)) }
        }

        impl<S: Clone + Send + Sync + 'static> MethodRouter<S> {
            pub fn $method<H, T>(mut self, handler: H) -> Self
            where H: Handler<T, S>, T: 'static,
            {
                self.inner = self.inner.$method(Adapter(handler));
                self
            }
        }
    )+};
}
methods!(get, post, put, patch, delete, head, options, trace);

impl<B> Service<http::Request<B>> for Router
where
    B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<super::body::BoxError>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Service::<axum::extract::Request>::poll_ready(&mut self.inner, cx)
    }

    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let future = Service::<axum::extract::Request>::call(
            &mut self.inner,
            request.map(axum::body::Body::new),
        );
        Box::pin(async move { future.await.map(|response| response.map(Body)) })
    }
}

/// The shared service passed to router layers. Backend route types stay private.
#[derive(Clone, Debug)]
pub struct Route(pub(crate) axum::routing::Route);
impl Service<Request> for Route {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Service::<axum::extract::Request>::poll_ready(&mut self.0, cx)
    }
    fn call(&mut self, request: Request) -> Self::Future {
        let future = self.0.call(request.map(|body| body.0));
        Box::pin(async move { future.await.map(|response| response.map(Body)) })
    }
}

pub fn any<H, T, S>(handler: H) -> MethodRouter<S>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    MethodRouter {
        inner: axum::routing::any(Adapter(handler)),
    }
}

/// Factory for servers that request one Tower service per accepted connection.
/// Connection targets are ignored; use `serve_with_connect_info` when peer
/// metadata is needed. TLS configuration and shutdown remain server-owned.
#[derive(Clone, Debug)]
pub struct MakeService(Router);
impl Router {
    pub fn into_make_service(self) -> MakeService {
        MakeService(self)
    }
}
impl<T> Service<T> for MakeService {
    type Response = Router;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Router, Infallible>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, _: T) -> Self::Future {
        std::future::ready(Ok(self.0.clone()))
    }
}

/// Register a standard Tower service for GET and HEAD.
pub fn get_service<T, B, S>(service: T) -> MethodRouter<S>
where
    T: Service<Request, Response = http::Response<B>, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    T::Future: Send + 'static,
    B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<super::body::BoxError>,
    S: Clone + Send + Sync + 'static,
{
    MethodRouter {
        inner: axum::routing::get_service(super::service::BackendService(service)),
    }
}

impl<S: Clone + Send + Sync + 'static> MethodRouter<S> {
    /// Supply this method group's state independently of its enclosing router.
    pub fn with_state<S2: Clone + Send + Sync + 'static>(self, state: S) -> MethodRouter<S2> {
        MethodRouter {
            inner: self.inner.with_state(state),
        }
    }

    /// Apply a Tower layer to this method router, including its method fallback.
    pub fn layer<L, B>(mut self, layer: L) -> Self
    where
        L: tower_layer::Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request, Response = http::Response<B>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.inner = self.inner.layer(super::service::BackendLayer(layer));
        self
    }

    /// Apply a layer to matched methods, preserving the unwrapped 405 fallback.
    pub fn route_layer<L, B>(mut self, layer: L) -> Self
    where
        L: tower_layer::Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request, Response = http::Response<B>, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.inner = self.inner.route_layer(super::service::BackendLayer(layer));
        self
    }
}
