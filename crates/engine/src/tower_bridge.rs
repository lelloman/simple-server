//! Stable native routes and request-extension snapshots for host Tower services.
use crate::{
    callback::{self, ForeignCallback},
    middleware::HostContext,
    operations::{FutureBytes, decode, encode, id},
    server::HostHandler,
};
use axum::{
    body::Body,
    extract::Request,
    handler::Handler,
    http::{Extensions, Response, Version},
    routing::Route,
};
use http_body::{Body as _, SizeHint};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    task::{Context, Poll, Waker},
};
use tower::{Layer, Service, ServiceExt};
pub const ROUTE: u32 = 16;
pub const EXTENSIONS: u32 = 17;
static ROUTES: OnceLock<Mutex<HashMap<u64, Route>>> = OnceLock::new();
static SNAPSHOTS: OnceLock<Mutex<HashMap<u64, Arc<Extensions>>>> = OnceLock::new();
fn routes() -> &'static Mutex<HashMap<u64, Route>> {
    ROUTES.get_or_init(Default::default)
}
fn snapshots() -> &'static Mutex<HashMap<u64, Arc<Extensions>>> {
    SNAPSHOTS.get_or_init(Default::default)
}
pub fn release(kind: u32, id: u64) {
    match kind {
        ROUTE => {
            let value = routes().lock().unwrap().remove(&id);
            drop(value);
        }
        EXTENSIONS => {
            let value = snapshots().lock().unwrap().remove(&id);
            drop(value);
        }
        _ => (),
    }
}
pub(crate) struct Borrowed(pub u32, pub u64);
impl Drop for Borrowed {
    fn drop(&mut self) {
        release(self.0, self.1);
    }
}
pub(crate) fn capture(extensions: Extensions) -> Borrowed {
    let id = id();
    snapshots().lock().unwrap().insert(id, Arc::new(extensions));
    Borrowed(EXTENSIONS, id)
}
fn route(id: u64) -> Result<Route, String> {
    routes()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "route released".into())
}
fn snapshot(id: u64) -> Result<Arc<Extensions>, String> {
    snapshots()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "native extensions released".into())
}
fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key].as_u64().ok_or_else(|| format!("missing {key}"))
}
pub fn resource_new(command: &Value) -> Result<Vec<u8>, String> {
    let id = id();
    if command["op"] == "tower_route_clone" {
        let value = route(number(command, "route")?)?;
        routes().lock().unwrap().insert(id, value);
    } else {
        let value = snapshot(number(command, "extensions")?)?;
        snapshots().lock().unwrap().insert(id, value);
    }
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
#[derive(Clone)]
pub struct HostLayer(pub Arc<ForeignCallback>);
impl Layer<Route> for HostLayer {
    type Service = HostService;
    fn layer(&self, inner: Route) -> Self::Service {
        let id = id();
        routes().lock().unwrap().insert(id, inner);
        let _borrowed = Borrowed(ROUTE, id);
        let result = (|| -> Result<HostService, String> {
            // Layer::layer is synchronous. The host factory never awaits; do not
            // block a runtime or silently accept an unexpectedly pending factory.
            let mut future = self.0.call(&encode(json!({"route":id}), &[]));
            let reply = match Pin::new(&mut future).poll(&mut Context::from_waker(Waker::noop())) {
                Poll::Ready(result) => result?,
                Poll::Pending => {
                    return Err("Tower layer factory must complete synchronously".into());
                }
            };
            let (header, _) = decode(&reply)?;
            if header["ok"] != true {
                return Err(header["message"]
                    .as_str()
                    .unwrap_or("Tower layer factory failed")
                    .into());
            }
            // Acquire the service before releasing its producer-owned reply.
            Ok(HostService(HostHandler(callback::get(number(
                &header, "handler",
            )?)?)))
        })();
        result.unwrap_or_else(|error| panic!("{error}"))
    }
}
#[derive(Clone)]
pub struct HostService(HostHandler);
impl Service<Request> for HostService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: Request) -> Self::Future {
        let handler = self.0.clone();
        Box::pin(async move { Ok(handler.call(request, ()).await) })
    }
}
pub fn operation(command: Value) -> Result<FutureBytes, String> {
    let route = route(number(&command, "route")?)?;
    let response_slot = crate::server::body(number(&command, "response")?)?;
    let extensions = command["extensions"].as_u64().map(snapshot).transpose()?;
    let lower = command["lower"].as_u64().unwrap_or(0);
    let upper = command["upper"].as_u64();
    if upper.is_some_and(|n| n < lower) {
        return Err("invalid body size hint".into());
    }
    let mut hint = SizeHint::new();
    hint.set_lower(lower);
    if let Some(upper) = upper {
        hint.set_upper(upper);
    }
    let body = crate::server::callback_body_with_hint(number(&command, "body")?, hint)?;
    let version = match command["version"].as_str() {
        Some("HTTP/0.9") => Version::HTTP_09,
        Some("HTTP/1.0") => Version::HTTP_10,
        Some("HTTP/1.1") => Version::HTTP_11,
        Some("HTTP/2.0") => Version::HTTP_2,
        Some("HTTP/3.0") => Version::HTTP_3,
        _ => return Err("invalid HTTP version".into()),
    };
    let mut request = Request::builder()
        .method(command["method"].as_str().ok_or("missing method")?)
        .uri(command["uri"].as_str().ok_or("missing URI")?)
        .version(version)
        .body(body)
        .map_err(|e| e.to_string())?;
    *request.headers_mut() = crate::server::headers_from_wire(&command["headers"])?;
    if let Some(extensions) = extensions {
        *request.extensions_mut() = (*extensions).clone();
    }
    request
        .extensions_mut()
        .insert(HostContext(number(&command, "context")?));
    Ok(Box::pin(async move {
        let response = route
            .oneshot(request)
            .await
            .unwrap_or_else(|never| match never {});
        let (parts, body) = response.into_parts();
        let hint = body.size_hint();
        let output = encode(
            json!({"ok":true,"status":parts.status.as_u16(),"version":format!("{:?}",parts.version),"headers":crate::server::headers_to_wire(&parts.headers),"lower":hint.lower(),"upper":hint.upper()}),
            &[],
        );
        *response_slot.lock().await = body;
        output
    }))
}
