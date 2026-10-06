//! Host extensions remain here, scoped to each active native continuation call.
use super::{Body, HttpBody, Request, Response, Service, body, wire};
use serde_json::json;
use simple_server_sys::{Operation, Resource};
use std::{
    collections::HashMap,
    convert::Infallible,
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
};
struct State {
    request: Option<http::Extensions>,
    response: http::Extensions,
}
static STATES: OnceLock<Mutex<HashMap<u64, Arc<Mutex<State>>>>> = OnceLock::new();
static IDS: AtomicU64 = AtomicU64::new(1);
fn states() -> &'static Mutex<HashMap<u64, Arc<Mutex<State>>>> {
    STATES.get_or_init(Default::default)
}
struct Guard {
    id: u64,
    state: Arc<Mutex<State>>,
}
impl Guard {
    fn new(extensions: http::Extensions) -> Self {
        let id = IDS.fetch_add(1, Ordering::Relaxed);
        let state = Arc::new(Mutex::new(State {
            request: Some(extensions),
            response: http::Extensions::new(),
        }));
        states().lock().unwrap().insert(id, state.clone());
        Self { id, state }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        let state = states().lock().unwrap().remove(&self.id);
        drop(state);
    }
}
pub(super) fn request_extensions(id: u64) -> http::Extensions {
    let state = states().lock().unwrap().get(&id).cloned();
    state
        .and_then(|state| state.lock().unwrap().request.take())
        .unwrap_or_default()
}
pub(super) fn response_extensions(id: u64, extensions: http::Extensions) {
    let state = states().lock().unwrap().get(&id).cloned();
    if let Some(state) = state {
        let old = std::mem::replace(&mut state.lock().unwrap().response, extensions);
        drop(old);
    }
}
#[cfg(feature = "engine-tracing")]
#[derive(Clone)]
pub(super) struct TraceContext(pub tracing::Span);

#[derive(Clone)]
pub(super) struct NativeExtensions(pub Resource);

/// A cloneable native route used as the inner service of a Tower layer.
/// Constructed by router/method layer registration; its native routing state
/// remains in the engine. Readiness is immediate; calls return streaming bodies.
#[derive(Clone)]
pub struct Route(pub(super) Resource);
impl Route {
    async fn run(self, request: Request) -> io::Result<Response> {
        let (mut parts, body) = request.into_parts();
        let native = parts.extensions.remove::<NativeExtensions>();
        #[cfg(feature = "engine-tracing")]
        parts
            .extensions
            .insert(TraceContext(tracing::Span::current()));
        let guard = Guard::new(parts.extensions);
        let hint = body.size_hint();
        let callback = body::export(body)?;
        let resource = wire::resource(6, json!({"op":"server_body_slot"}))?;
        let output = Operation::new(&wire::encode(json!({"op":"tower_route_call","route":self.0.id(),"extensions":native.as_ref().map(|extensions| extensions.0.id()),"context":guard.id,"response":resource.id(),"method":parts.method.as_str(),"uri":parts.uri.to_string(),"version":format!("{:?}",parts.version),"headers":wire::headers(&parts.headers),"body":callback.id(),"lower":hint.lower(),"upper":hint.upper()}), &[])?)?.await?;
        let (header, _) = wire::decode(&output)?;
        wire::check(&header)?;
        let status = header["status"]
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .and_then(|n| http::StatusCode::from_u16(n).ok())
            .ok_or_else(|| io::Error::other("invalid response status"))?;
        let mut response = Response::new(Body::incoming(resource, &header)?);
        *response.status_mut() = status;
        *response.headers_mut() = wire::read_headers(&header["headers"])?;
        *response.version_mut() = wire::version(&header)?;
        *response.extensions_mut() = std::mem::take(&mut guard.state.lock().unwrap().response);
        Ok(response)
    }
}
impl Service<Request> for Route {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: Request) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            Ok(this.run(request).await.unwrap_or_else(|_| {
                let mut response = Response::new(Body::from("internal server error"));
                *response.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                response
            }))
        })
    }
}
