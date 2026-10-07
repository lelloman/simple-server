//! Native SHA-256 state. Only byte buffers and owned numeric IDs cross the ABI.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
pub const HASHER: u32 = 22;
type State = Arc<Mutex<Option<Sha256>>>;
static STATES: OnceLock<Mutex<HashMap<u64, State>>> = OnceLock::new();
fn states() -> &'static Mutex<HashMap<u64, State>> {
    STATES.get_or_init(Default::default)
}
pub fn release(id: u64) {
    let state = states().lock().unwrap().remove(&id);
    drop(state);
}
pub fn execute(command: &Value, payload: &[u8]) -> Result<Vec<u8>, String> {
    let op = command["op"].as_str().ok_or("invalid SHA-256 command")?;
    if op == "sha256_digest" {
        return Ok(crate::operations::encode(
            json!({}),
            &Sha256::digest(payload),
        ));
    }
    if op == "sha256_new" {
        let id = crate::operations::id();
        states()
            .lock()
            .unwrap()
            .insert(id, Arc::new(Mutex::new(Some(Sha256::new()))));
        return Ok(crate::operations::encode(json!({"id":id}), &[]));
    }
    if !matches!(op, "sha256_update" | "sha256_finalize") {
        return Err("unknown SHA-256 command".into());
    }
    let id = command["id"]
        .as_u64()
        .filter(|id| *id != 0)
        .ok_or("invalid SHA-256 handle")?;
    let state = if op == "sha256_finalize" {
        states().lock().unwrap().remove(&id)
    } else {
        states().lock().unwrap().get(&id).cloned()
    }
    .ok_or("unknown SHA-256 handle")?;
    let mut state = state.lock().unwrap();
    if op == "sha256_update" {
        state
            .as_mut()
            .ok_or("finalized SHA-256 handle")?
            .update(payload);
        Ok(crate::operations::encode(json!({}), &[]))
    } else {
        let digest = state.take().ok_or("finalized SHA-256 handle")?.finalize();
        Ok(crate::operations::encode(json!({}), &digest))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn new_id() -> u64 {
        let response = execute(&json!({"op":"sha256_new"}), &[]).unwrap();
        crate::operations::decode(&response).unwrap().0["id"]
            .as_u64()
            .unwrap()
    }
    #[test]
    fn finalize_and_abandon_release_state_and_reject_stale_handles() {
        for finish in [true, false] {
            let id = new_id();
            execute(&json!({"op":"sha256_update","id":id}), b"abc").unwrap();
            if finish {
                execute(&json!({"op":"sha256_finalize","id":id}), &[]).unwrap();
            } else {
                release(id);
            }
            assert!(!states().lock().unwrap().contains_key(&id));
            assert!(
                execute(&json!({"op":"sha256_update","id":id}), b"private bytes")
                    .unwrap_err()
                    .contains("unknown SHA-256 handle")
            );
            assert!(execute(&json!({"op":"sha256_finalize","id":id}), &[]).is_err());
            release(id); // Drop after finalize is harmless.
        }
        assert!(execute(&json!({"op":"sha256_update","id":0}), &[]).is_err());
    }
}
