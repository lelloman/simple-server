use crate::operations::{FutureBytes, encode, id};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

pub const SIGNALS: u32 = 10;
#[cfg(unix)]
struct Registration {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}
#[cfg(not(unix))]
struct Registration;
type Registrations = Mutex<HashMap<u64, Arc<tokio::sync::Mutex<Registration>>>>;
fn registrations() -> &'static Registrations {
    static REGISTRATIONS: OnceLock<Registrations> = OnceLock::new();
    REGISTRATIONS.get_or_init(Default::default)
}
pub fn install() -> Result<Vec<u8>, String> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let value = Registration {
            interrupt: signal(SignalKind::interrupt()).map_err(|e| e.to_string())?,
            terminate: signal(SignalKind::terminate()).map_err(|e| e.to_string())?,
        };
        let id = id();
        registrations()
            .lock()
            .unwrap()
            .insert(id, Arc::new(tokio::sync::Mutex::new(value)));
        Ok(encode(json!({"ok":true}), &id.to_le_bytes()))
    }
    #[cfg(not(unix))]
    Err("engine shutdown signals require Unix".into())
}
pub fn release(id: u64) {
    let registration = registrations().lock().unwrap().remove(&id);
    drop(registration);
}
pub fn wait(id: u64) -> Result<FutureBytes, String> {
    let registration = registrations()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or("signal registration released")?;
    Ok(Box::pin(async move {
        #[cfg(unix)]
        {
            let mut registration = registration.lock().await;
            let Registration {
                interrupt,
                terminate,
            } = &mut *registration;
            tokio::select! {
                value = interrupt.recv() => vec![if value.is_some() { 1 } else { 0 }],
                value = terminate.recv() => vec![if value.is_some() { 2 } else { 0 }],
            }
        }
        #[cfg(not(unix))]
        {
            drop(registration);
            vec![0]
        }
    }))
}
