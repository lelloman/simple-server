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

pub(super) fn layer_factory<L, B>(layer: L) -> std::io::Result<simple_server_sys::Callback>
where
    L: tower_layer::Layer<super::Route> + Clone + Send + Sync + 'static,
    L::Service: Service<Request, Response = http::Response<B>, Error = Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    <L::Service as Service<Request>>::Future: Send + 'static,
    B: http_body::Body<Data = super::Bytes> + Send + 'static,
    B::Error: Into<super::body::BoxError>,
{
    use super::wire;
    use serde_json::json;
    use simple_server_sys::{Callback, Reply};
    let host = super::host_runtime::HostRuntime::capture();
    let layer = std::sync::Arc::new(host.own(layer));
    Callback::with_reply(move |bytes| {
        let layer = (**layer).clone();
        host.scope(async move {
            // This factory must remain synchronous when polled. Layer::layer is
            // called once for each native route, never in the request callback.
            let result = (|| -> std::io::Result<Reply> {
                let (header, _) = wire::decode(&bytes)?;
                let route = wire::resource(
                    16,
                    json!({"op":"tower_route_clone","route":header["route"]}),
                )?;
                let service = layer.layer(super::Route(route));
                let handler =
                    super::routing::callback(move |request| call(service.clone(), request))?;
                Ok(Reply::new(wire::encode(
                    json!({"ok":true,"handler":handler.id()}),
                    &[],
                )?)
                .keep_alive(handler))
            })();
            result.unwrap_or_else(|error| {
                Reply::new(
                    wire::encode(json!({"ok":false,"message":error.to_string()}), &[])
                        .expect("factory error"),
                )
            })
        })
    })
}
