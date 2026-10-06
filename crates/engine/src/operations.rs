use serde_json::{Value, json};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

pub(crate) type FutureBytes = Pin<Box<dyn Future<Output = Vec<u8>> + Send>>;
static NEXT: AtomicU64 = AtomicU64::new(1);
static CLIENTS: OnceLock<Mutex<HashMap<u64, reqwest::Client>>> = OnceLock::new();
type ResponseResources = Mutex<HashMap<u64, Arc<tokio::sync::Mutex<Option<reqwest::Response>>>>>;
static RESPONSES: OnceLock<ResponseResources> = OnceLock::new();
fn clients() -> &'static Mutex<HashMap<u64, reqwest::Client>> {
    CLIENTS.get_or_init(Default::default)
}
fn responses() -> &'static ResponseResources {
    RESPONSES.get_or_init(Default::default)
}
pub(crate) fn id() -> u64 {
    NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("engine resource IDs exhausted")
}

pub fn encode(header: Value, payload: &[u8]) -> Vec<u8> {
    let header = serde_json::to_vec(&header).expect("engine metadata serialization");
    let length = u32::try_from(header.len()).expect("engine metadata exceeds framing limit");
    let mut bytes = Vec::with_capacity(8 + header.len() + payload.len());
    bytes.extend_from_slice(b"SS01");
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(payload);
    bytes
}
pub fn decode(bytes: &[u8]) -> Result<(Value, Vec<u8>), String> {
    if !bytes.starts_with(b"SS01") {
        return serde_json::from_slice(bytes)
            .map(|v| (v, Vec::new()))
            .map_err(|e| e.to_string());
    }
    if bytes.len() < 8 {
        return Err("truncated engine command".into());
    }
    let end = 8_usize
        .checked_add(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize)
        .filter(|end| *end <= bytes.len())
        .ok_or("invalid engine command length")?;
    let command = serde_json::from_slice(&bytes[8..end]).map_err(|e| e.to_string())?;
    Ok((command, bytes[end..].to_vec()))
}
fn failure(error: reqwest::Error) -> Vec<u8> {
    encode(
        json!({"ok":false,"message":error.to_string(),"timeout":error.is_timeout(),"connect":error.is_connect()}),
        &[],
    )
}
fn number(command: &Value, key: &str) -> Result<u64, String> {
    command[key]
        .as_u64()
        .ok_or_else(|| format!("missing {key}"))
}

pub fn resource_new(command: Value, _: Vec<u8>) -> Result<Vec<u8>, String> {
    match command["op"].as_str() {
        Some("tower_route_clone" | "tower_extensions_clone") => {
            crate::tower_bridge::resource_new(&command)
        }
        Some("middleware_next_clone") => crate::middleware::resource_new(&command),
        Some("shutdown_signals") => crate::signals::install(),
        Some("multipart_new" | "multipart_field_slot") => crate::multipart::resource_new(&command),
        #[cfg(unix)]
        Some("unix_client") => crate::unix_client::new(),
        Some("server_body_clone" | "server_body_slot") => crate::server::resource_new(&command),
        Some(operation) if operation.starts_with("router_") || operation.starts_with("method_") => {
            crate::routing::resource_new(&command)
        }
        Some("postgres_pool") => crate::postgres::connect_lazy(command),
        Some("client") => {
            let mut builder = reqwest::Client::builder();
            if let Some(value) = command["https_only"].as_bool() {
                builder = builder.https_only(value);
            }
            if let Some(timeout) = command["timeout_ms"].as_u64() {
                builder = builder.timeout(Duration::from_millis(timeout));
            }
            if let Some(timeout) = command["connect_timeout_ms"].as_u64() {
                builder = builder.connect_timeout(Duration::from_millis(timeout));
            }
            if let Some(timeout) = command["read_timeout_ms"].as_u64() {
                builder = builder.read_timeout(Duration::from_millis(timeout));
            }
            if let Some(agent) = command["user_agent"].as_str() {
                builder = builder.user_agent(agent);
            }
            if command["no_proxy"].as_bool() == Some(true) {
                builder = builder.no_proxy();
            }
            if command["no_redirect"].as_bool() == Some(true) {
                builder = builder.redirect(reqwest::redirect::Policy::none());
            }
            if command["http2_prior_knowledge"].as_bool() == Some(true) {
                builder = builder.http2_prior_knowledge();
            }
            let client = builder.build().map_err(|e| e.to_string())?;
            let id = id();
            clients().lock().unwrap().insert(id, client);
            Ok(encode(json!({"ok":true,"id":id}), &[]))
        }
        _ => Err("unknown engine resource".into()),
    }
}
pub fn resource_release(kind: u32, id: u64) {
    match kind {
        crate::multipart::PARSER | crate::multipart::FIELD => crate::multipart::release(kind, id),
        #[cfg(unix)]
        crate::unix_client::CLIENT => crate::unix_client::release(id),
        crate::signals::SIGNALS => crate::signals::release(id),
        1 => {
            clients().lock().unwrap().remove(&id);
        }
        2 => {
            responses().lock().unwrap().remove(&id);
        }
        5 => crate::callback::remove(id),
        #[cfg(unix)]
        crate::server::UNIX_LISTENER => crate::server::release(kind, id),
        crate::server::BODY | crate::server::LISTENER => crate::server::release(kind, id),
        crate::routing::ROUTER | crate::routing::METHODS => crate::routing::release(kind, id),
        crate::middleware::CONTINUATION => crate::middleware::release(id),
        crate::tower_bridge::ROUTE | crate::tower_bridge::EXTENSIONS => {
            crate::tower_bridge::release(kind, id)
        }
        _ => {
            crate::database::release(kind, id);
            crate::postgres::release(kind, id);
        }
    }
}

pub fn operation(command: Value, body: Vec<u8>) -> Result<FutureBytes, String> {
    match command["op"].as_str() {
        Some("tower_route_call") => crate::tower_bridge::operation(command),
        Some("middleware_next") => crate::middleware::operation(command),
        Some(op) if op.starts_with("multipart_") => crate::multipart::operation(command),
        #[cfg(unix)]
        Some("unix_request") => crate::unix_client::request(command),
        Some("shutdown_signal_wait") => crate::signals::wait(number(&command, "id")?),
        Some(
            "server_bind" | "server_serve" | "server_body_frame" | "server_unix_bind"
            | "server_unix_serve",
        ) => crate::server::operation(command, body),
        Some("callback") => {
            let callback = crate::callback::get(number(&command, "callback")?)?;
            Ok(Box::pin(async move {
                match callback.call(&body).await {
                    Ok(bytes) => encode(json!({"ok":true}), &bytes),
                    Err(error) => encode(json!({"ok":false,"message":error}), &[]),
                }
            }))
        }
        Some("postgres") => Ok(Box::pin(crate::postgres::run(command, body))),
        Some("sqlite") => Ok(Box::pin(crate::database::run(command, body))),
        Some("http_send") => {
            let client = clients()
                .lock()
                .unwrap()
                .get(&number(&command, "client")?)
                .cloned()
                .ok_or("HTTP client already released")?;
            let method = reqwest::Method::from_bytes(
                command["method"]
                    .as_str()
                    .ok_or("missing method")?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
            let url = command["url"].as_str().ok_or("missing URL")?;
            let mut request = client.request(method, url).body(body);
            if let Some(headers) = command["headers"].as_array() {
                for pair in headers {
                    let name = pair[0].as_str().ok_or("invalid header name")?;
                    let bytes: Vec<u8> =
                        serde_json::from_value(pair[1].clone()).map_err(|e| e.to_string())?;
                    let value = reqwest::header::HeaderValue::from_bytes(&bytes)
                        .map_err(|e| e.to_string())?;
                    request = request.header(name, value);
                }
            }
            if let Some(timeout) = command["timeout_ms"].as_u64() {
                request = request.timeout(Duration::from_millis(timeout));
            }
            Ok(Box::pin(async move {
                match request.send().await {
                    Err(error) => failure(error),
                    Ok(response) => {
                        let id = id();
                        let headers: Vec<_> = response
                            .headers()
                            .iter()
                            .map(|(k, v)| json!([k.as_str(), v.as_bytes()]))
                            .collect();
                        let metadata = json!({"ok":true,"id":id,"status":response.status().as_u16(),"headers":headers,"url":response.url().as_str(),"content_length":response.content_length(),"version":format!("{:?}",response.version())});
                        responses()
                            .lock()
                            .unwrap()
                            .insert(id, Arc::new(tokio::sync::Mutex::new(Some(response))));
                        encode(metadata, &[])
                    }
                }
            }))
        }
        Some("http_chunk") => {
            let response = responses()
                .lock()
                .unwrap()
                .get(&number(&command, "response")?)
                .cloned()
                .ok_or("HTTP response already released")?;
            Ok(Box::pin(async move {
                let mut guard = response.lock().await;
                let Some(response) = guard.as_mut() else {
                    return encode(
                        json!({"ok":false,"message":"response already consumed"}),
                        &[],
                    );
                };
                match response.chunk().await {
                    Ok(Some(bytes)) => encode(json!({"ok":true,"eof":false}), &bytes),
                    Ok(None) => encode(json!({"ok":true,"eof":true}), &[]),
                    Err(error) => failure(error),
                }
            }))
        }
        Some("http_text") => {
            let response = responses()
                .lock()
                .unwrap()
                .get(&number(&command, "response")?)
                .cloned()
                .ok_or("HTTP response already released")?;
            Ok(Box::pin(async move {
                let Some(response) = response.lock().await.take() else {
                    return encode(
                        json!({"ok":false,"message":"response already consumed"}),
                        &[],
                    );
                };
                match response.text().await {
                    Ok(text) => encode(json!({"ok":true}), text.as_bytes()),
                    Err(error) => failure(error),
                }
            }))
        }
        Some("process_output") => {
            use std::os::unix::ffi::OsStringExt;
            let executable: Vec<u8> = serde_json::from_value(command["program_bytes"].clone())
                .map_err(|e| e.to_string())?;
            let executable = std::ffi::OsString::from_vec(executable);
            let args: Vec<Vec<u8>> =
                serde_json::from_value(command["args_bytes"].clone()).map_err(|e| e.to_string())?;
            let args: Vec<_> = args.into_iter().map(std::ffi::OsString::from_vec).collect();
            Ok(Box::pin(async move {
                use std::os::unix::process::ExitStatusExt;
                match tokio::process::Command::new(executable)
                    .args(args)
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout_len = output.stdout.len();
                        let mut body = output.stdout;
                        body.extend(output.stderr);
                        encode(
                            json!({"ok":true,"status":output.status.into_raw(),"stdout_len":stdout_len}),
                            &body,
                        )
                    }
                    Err(error) => encode(
                        json!({"ok":false,"message":error.to_string(),"os_error":error.raw_os_error()}),
                        &[],
                    ),
                }
            }))
        }
        _ => Err("unsupported engine operation".into()),
    }
}
