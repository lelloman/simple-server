use super::{Body, Request, routing::Route};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_service::Service;

pub(super) fn request(mut request: axum::extract::Request) -> Request {
    if let Some(path) = request.extensions().get::<axum::extract::MatchedPath>() {
        let path = super::MatchedPath(path.as_str().into());
        request.extensions_mut().insert(path);
    }
    if let Some(peer) = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        let peer = super::ConnectInfo(peer.0);
        request.extensions_mut().insert(peer);
    }
    request.map(Body)
}

#[derive(Clone)]
pub(super) struct BackendLayer<L>(pub L);
impl<L: tower_layer::Layer<Route>> tower_layer::Layer<axum::routing::Route> for BackendLayer<L> {
    type Service = BackendService<L::Service>;
    fn layer(&self, inner: axum::routing::Route) -> Self::Service {
        BackendService(self.0.layer(Route(inner)))
    }
}

#[derive(Clone)]
pub(super) struct BackendService<T>(pub T);
impl<T, B> Service<axum::extract::Request> for BackendService<T>
where
    T: Service<Request, Response = http::Response<B>, Error = Infallible>,
    T::Future: Send + 'static,
    B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<super::body::BoxError>,
{
    type Response = axum::response::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.0.poll_ready(cx)
    }
    fn call(&mut self, req: axum::extract::Request) -> Self::Future {
        let future = self.0.call(request(req));
        Box::pin(async move {
            future
                .await
                .map(|response| response.map(axum::body::Body::new))
        })
    }
}
