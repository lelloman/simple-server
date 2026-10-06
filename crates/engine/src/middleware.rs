//! Native route continuations. Only resource/context IDs cross the ABI.
use crate::{
    callback::ForeignCallback,
    operations::{FutureBytes, encode, id},
    server,
};
use axum::{
    body::Body,
    extract::Request,
    http::{Extensions, Response, StatusCode, Version},
    middleware::Next,
};
use http_body::{Body as _, SizeHint};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
pub const CONTINUATION: u32 = 15;
#[derive(Clone)]
pub(crate) struct HostContext(pub u64);
struct Continuation {
    next: Next,
    extensions: Extensions,
}
static CONTINUATIONS: OnceLock<Mutex<HashMap<u64, Arc<Continuation>>>> = OnceLock::new();
fn registry() -> &'static Mutex<HashMap<u64, Arc<Continuation>>> {
    CONTINUATIONS.get_or_init(Default::default)
}
pub fn release(id: u64) {
    let value = registry().lock().unwrap().remove(&id);
    drop(value);
}
struct Borrowed(u64);
impl Drop for Borrowed {
    fn drop(&mut self) {
        release(self.0);
    }
}
fn get(id: u64) -> Result<Arc<Continuation>, String> {
    registry()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "continuation released".into())
}
fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key].as_u64().ok_or_else(|| format!("missing {key}"))
}
pub fn resource_new(command: &Value) -> Result<Vec<u8>, String> {
    let value = get(number(command, "continuation")?)?;
    let id = id();
    registry().lock().unwrap().insert(id, value);
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
pub async fn run(request: Request, next: Next, callback: Arc<ForeignCallback>) -> Response<Body> {
    let id = id();
    let value = Continuation {
        next,
        extensions: request.extensions().clone(),
    };
    registry().lock().unwrap().insert(id, Arc::new(value));
    let _borrowed = Borrowed(id);
    server::forward_with_next(request, callback, Some(id))
        .await
        .unwrap_or_else(|_| {
            let mut response = Response::new(Body::from("internal server error"));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
        })
}
pub fn operation(command: Value) -> Result<FutureBytes, String> {
    let continuation = get(number(&command, "continuation")?)?;
    let response_slot = server::body(number(&command, "response")?)?;
    let mut hint = SizeHint::new();
    let lower = command["lower"].as_u64().unwrap_or(0);
    let upper = command["upper"].as_u64();
    if upper.is_some_and(|upper| upper < lower) {
        return Err("invalid body size hint".into());
    }
    hint.set_lower(lower);
    if let Some(upper) = upper {
        hint.set_upper(upper);
    }
    let body = server::callback_body_with_hint(number(&command, "body")?, hint)?;
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
    *request.headers_mut() = server::headers_from_wire(&command["headers"])?;
    *request.extensions_mut() = continuation.extensions.clone();
    request
        .extensions_mut()
        .insert(HostContext(number(&command, "context")?));
    Ok(Box::pin(async move {
        let response = continuation.next.clone().run(request).await;
        let (parts, body) = response.into_parts();
        let hint = body.size_hint();
        let output = encode(
            json!({"ok":true,"status":parts.status.as_u16(),"version":format!("{:?}",parts.version),"headers":server::headers_to_wire(&parts.headers),"lower":hint.lower(),"upper":hint.upper()}),
            &[],
        );
        *response_slot.lock().await = body;
        output
    }))
}
