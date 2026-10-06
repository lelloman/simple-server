//! Unix HTTP pooling and transport remain native; bodies cross the existing
//! streaming callback/resource boundary. Response slots predate async work.
use crate::{
    operations::{FutureBytes, encode, id},
    server,
};
use axum::{
    body::Body,
    http::{Request, Version},
};
use http_body::{Body as _, SizeHint};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
pub const CLIENT: u32 = 14;
type UnixClient = Client<UnixConnector, Body>;
static CLIENTS: OnceLock<Mutex<HashMap<u64, UnixClient>>> = OnceLock::new();
fn clients() -> &'static Mutex<HashMap<u64, UnixClient>> {
    CLIENTS.get_or_init(Default::default)
}
pub fn new() -> Result<Vec<u8>, String> {
    let client = UnixClient::unix();
    let id = id();
    clients().lock().unwrap().insert(id, client);
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
pub fn release(id: u64) {
    let client = clients().lock().unwrap().remove(&id);
    drop(client);
}
fn number(command: &Value, key: &str) -> Result<u64, String> {
    command[key]
        .as_u64()
        .ok_or_else(|| format!("missing {key}"))
}
pub fn request(command: Value) -> Result<FutureBytes, String> {
    let client = clients()
        .lock()
        .unwrap()
        .get(&number(&command, "client")?)
        .cloned()
        .ok_or("Unix client released")?;
    let response_slot = server::body(number(&command, "response")?)?;
    let mut hint = SizeHint::new();
    let lower = command["lower"].as_u64().unwrap_or(0);
    let upper = command["upper"].as_u64();
    if upper.is_some_and(|upper| upper < lower) {
        return Err("invalid request body size hint".into());
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
    Ok(Box::pin(async move {
        match client.request(request).await {
            Ok(response) => {
                let (parts, body) = response.into_parts();
                let hint = body.size_hint();
                let output = encode(
                    json!({"ok":true,"status":parts.status.as_u16(),"version":format!("{:?}",parts.version),"headers":server::headers_to_wire(&parts.headers),"lower":hint.lower(),"upper":hint.upper()}),
                    &[],
                );
                *response_slot.lock().await = Body::new(body);
                output
            }
            Err(error) => encode(json!({"ok":false,"message":error.to_string()}), &[]),
        }
    }))
}
