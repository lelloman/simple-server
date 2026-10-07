//! WebSocket handshakes and full-duplex transport stay in the native engine.
use crate::{
    callback,
    operations::{FutureBytes, encode, id},
    server::{headers_from_wire, headers_to_wire},
};
use axum::{
    body::Body,
    extract::{
        FromRequestParts,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    http::{Request, Version},
};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
type Upgrade = WebSocketUpgrade;
type Socket = Arc<(
    tokio::sync::Mutex<SplitSink<WebSocket, Message>>,
    tokio::sync::Mutex<SplitStream<WebSocket>>,
)>;
fn upgrades() -> &'static Mutex<HashMap<u64, Option<Upgrade>>> {
    static MAP: OnceLock<Mutex<HashMap<u64, Option<Upgrade>>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}
fn sockets() -> &'static Mutex<HashMap<u64, Socket>> {
    static MAP: OnceLock<Mutex<HashMap<u64, Socket>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}
pub fn release(kind: u32, id: u64) {
    if kind == 24 {
        let value = upgrades().lock().unwrap().remove(&id);
        drop(value);
    } else {
        let value = sockets().lock().unwrap().remove(&id);
        drop(value);
    }
}
struct BorrowedSocket(u64);
impl Drop for BorrowedSocket {
    fn drop(&mut self) {
        release(25, self.0);
    }
}
fn number(c: &Value, k: &str) -> Result<u64, String> {
    c[k].as_u64().ok_or_else(|| format!("missing {k}"))
}
pub fn resource_new(c: &Value) -> Result<Vec<u8>, String> {
    match c["op"].as_str() {
        Some("ws_upgrade_slot") => {
            let id = id();
            upgrades().lock().unwrap().insert(id, None);
            Ok(encode(json!({"ok":true,"id":id}), &[]))
        }
        Some("ws_socket_clone") => {
            let socket = sockets()
                .lock()
                .unwrap()
                .get(&number(c, "socket")?)
                .cloned()
                .ok_or("socket released")?;
            let id = id();
            sockets().lock().unwrap().insert(id, socket);
            Ok(encode(json!({"ok":true,"id":id}), &[]))
        }
        Some("ws_accept") => accept(c),
        _ => Err("unknown WebSocket resource command".into()),
    }
}
fn accept(c: &Value) -> Result<Vec<u8>, String> {
    let callback = callback::get(number(c, "callback")?)?;
    let mut ws = upgrades()
        .lock()
        .unwrap()
        .remove(&number(c, "upgrade")?)
        .flatten()
        .ok_or("upgrade released or consumed")?;
    for (key, setter) in [
        (
            "read_buffer_size",
            Upgrade::read_buffer_size as fn(Upgrade, usize) -> Upgrade,
        ),
        ("write_buffer_size", Upgrade::write_buffer_size),
        ("max_write_buffer_size", Upgrade::max_write_buffer_size),
        ("max_message_size", Upgrade::max_message_size),
        ("max_frame_size", Upgrade::max_frame_size),
    ] {
        if let Some(n) = c[key].as_u64() {
            ws = setter(ws, usize::try_from(n).map_err(|e| e.to_string())?);
        }
    }
    if let Some(accept) = c["accept_unmasked_frames"].as_bool() {
        ws = ws.accept_unmasked_frames(accept);
    }
    if let Some(protocol) = c["selected_protocol"].as_str() {
        ws.set_selected_protocol(
            protocol
                .parse()
                .map_err(|e: axum::http::header::InvalidHeaderValue| e.to_string())?,
        );
    }
    let failed = callback.clone();
    let response = ws
        .on_failed_upgrade(move |error: axum::Error| {
            tokio::spawn(async move {
                let _ = failed
                    .call(&encode(
                        json!({"ok":false,"message":error.to_string()}),
                        &[],
                    ))
                    .await;
            });
        })
        .on_upgrade(move |socket| async move {
            let id = id();
            let (sink, stream) = socket.split();
            sockets().lock().unwrap().insert(
                id,
                Arc::new((
                    tokio::sync::Mutex::new(sink),
                    tokio::sync::Mutex::new(stream),
                )),
            );
            let _borrowed = BorrowedSocket(id);
            let _ = callback
                .call(&encode(json!({"ok":true,"socket":id}), &[]))
                .await;
        });
    Ok(encode(
        json!({"ok":true,"status":response.status().as_u16(),"headers":headers_to_wire(response.headers())}),
        &[],
    ))
}
pub fn operation(c: Value, body: Vec<u8>) -> Result<FutureBytes, String> {
    if c["op"] == "ws_extract" {
        let mut request = Request::builder()
            .method(c["method"].as_str().ok_or("missing method")?)
            .body(Body::empty())
            .map_err(|e| e.to_string())?;
        *request.headers_mut() = headers_from_wire(&c["headers"])?;
        *request.version_mut() = if c["version"] == "HTTP/2.0" {
            Version::HTTP_2
        } else {
            Version::HTTP_11
        };
        *request.extensions_mut() =
            (*crate::tower_bridge::snapshot(number(&c, "extensions")?)?).clone();
        let slot = number(&c, "upgrade")?;
        return Ok(Box::pin(async move {
            let (mut parts, _) = request.into_parts();
            match Upgrade::from_request_parts(&mut parts, &()).await {
                Ok(ws) => {
                    if let Some(target) = upgrades().lock().unwrap().get_mut(&slot) {
                        *target = Some(ws);
                        encode(json!({"ok":true}), &[])
                    } else {
                        encode(json!({"ok":false,"message":"upgrade released"}), &[])
                    }
                }
                Err(e) => encode(
                    json!({"ok":false,"status":e.status().as_u16(),"message":e.body_text()}),
                    &[],
                ),
            }
        }));
    }
    let socket = sockets()
        .lock()
        .unwrap()
        .get(&number(&c, "socket")?)
        .cloned()
        .ok_or("socket released")?;
    Ok(Box::pin(async move {
        let result: Result<Vec<u8>, String> = async {
            match c["op"].as_str() {
                Some("ws_recv") => {
                    let message = socket
                        .1
                        .lock()
                        .await
                        .next()
                        .await
                        .transpose()
                        .map_err(|e| e.to_string())?;
                    let (kind, data, code) = match message {
                        None => ("end", Vec::new(), None),
                        Some(Message::Text(t)) => ("text", t.as_bytes().to_vec(), None),
                        Some(Message::Binary(b)) => ("binary", b.to_vec(), None),
                        Some(Message::Ping(b)) => ("ping", b.to_vec(), None),
                        Some(Message::Pong(b)) => ("pong", b.to_vec(), None),
                        Some(Message::Close(f)) => (
                            "close",
                            f.as_ref()
                                .map(|f| f.reason.as_bytes().to_vec())
                                .unwrap_or_default(),
                            f.map(|f| f.code),
                        ),
                    };
                    Ok(encode(json!({"ok":true,"kind":kind,"code":code}), &data))
                }
                Some("ws_send") => {
                    let message = match c["kind"].as_str() {
                        Some("text") => Message::Text(
                            String::from_utf8(body).map_err(|e| e.to_string())?.into(),
                        ),
                        Some("binary") => Message::Binary(body.into()),
                        Some("ping") => Message::Ping(body.into()),
                        Some("pong") => Message::Pong(body.into()),
                        Some("close") => Message::Close(
                            c["code"]
                                .as_u64()
                                .map(|code| -> Result<CloseFrame, String> {
                                    Ok(CloseFrame {
                                        code: u16::try_from(code).map_err(|e| e.to_string())?,
                                        reason: String::from_utf8(body)
                                            .map_err(|e| e.to_string())?
                                            .into(),
                                    })
                                })
                                .transpose()?,
                        ),
                        _ => return Err("unknown WebSocket message".into()),
                    };
                    socket
                        .0
                        .lock()
                        .await
                        .send(message)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(encode(json!({"ok":true}), &[]))
                }
                Some("ws_flush") => {
                    socket
                        .0
                        .lock()
                        .await
                        .flush()
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(encode(json!({"ok":true}), &[]))
                }
                Some("ws_close") => {
                    socket
                        .0
                        .lock()
                        .await
                        .close()
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(encode(json!({"ok":true}), &[]))
                }
                _ => Err("unknown WebSocket operation".into()),
            }
        }
        .await;
        result.unwrap_or_else(|message| encode(json!({"ok":false,"message":message}), &[]))
    }))
}
