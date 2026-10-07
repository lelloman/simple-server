//! Repeatable Unix signal receivers owned by the engine.
use crate::operations::{FutureBytes, encode, id};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
pub const SIGNAL: u32 = 23;
type Receiver = Arc<tokio::sync::Mutex<tokio::signal::unix::Signal>>;
fn receivers() -> &'static Mutex<HashMap<u64, Receiver>> {
    static RECEIVERS: OnceLock<Mutex<HashMap<u64, Receiver>>> = OnceLock::new();
    RECEIVERS.get_or_init(Default::default)
}
pub fn install(command: &Value) -> Result<Vec<u8>, String> {
    use tokio::signal::unix::{SignalKind, signal};
    let kind = match command["kind"].as_str() {
        Some("hangup") => SignalKind::hangup(),
        Some("interrupt") => SignalKind::interrupt(),
        Some("terminate") => SignalKind::terminate(),
        _ => return Err("unsupported Unix signal kind".into()),
    };
    let receiver = signal(kind).map_err(|e| e.to_string())?;
    let id = id();
    receivers()
        .lock()
        .unwrap()
        .insert(id, Arc::new(tokio::sync::Mutex::new(receiver)));
    Ok(encode(json!({"ok":true}), &id.to_le_bytes()))
}
pub fn release(id: u64) {
    let receiver = receivers().lock().unwrap().remove(&id);
    drop(receiver);
}
pub fn wait(id: u64) -> Result<FutureBytes, String> {
    let receiver = receivers()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or("Unix signal receiver released")?;
    Ok(Box::pin(async move {
        vec![u8::from(receiver.lock().await.recv().await.is_some())]
    }))
}
