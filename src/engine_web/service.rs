//! Host-side service dispatch; neither service state nor readiness crosses the ABI.
use super::{Body, Request, Response, Service};
use std::{convert::Infallible, future::poll_fn};

pub(super) async fn call<T, B>(mut service: T, request: Request) -> Response
where
    T: Service<Request, Response = http::Response<B>, Error = Infallible> + Send,
    T::Future: Send,
    B: http_body::Body<Data = super::Bytes> + Send + 'static,
    B::Error: Into<super::body::BoxError>,
{
    // Do not clone after poll_ready: readiness may reserve capacity in this instance.
    match poll_fn(|cx| service.poll_ready(cx)).await {
        Ok(()) => {}
        Err(error) => match error {},
    }
    match service.call(request).await {
        Ok(response) => response.map(Body::new),
        Err(error) => match error {},
    }
}
