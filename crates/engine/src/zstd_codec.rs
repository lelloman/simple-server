//! Synchronous buffer codecs, usable from host blocking workers without a runtime.
use serde_json::{Value, json};
use std::io::{self, Read};

pub(crate) fn execute(command: &Value, input: &[u8]) -> Result<Vec<u8>, String> {
    let result = match command["op"].as_str() {
        Some("zstd_encode") => {
            let level = command["level"]
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or("invalid Zstd level")?;
            zstd::stream::encode_all(input, level)
        }
        Some("zstd_decode") => {
            let limit = match command.get("max_bytes") {
                Some(value) => Some(value.as_u64().ok_or("invalid Zstd output limit")?),
                None => None,
            };
            decode(input, limit)
        }
        _ => return Err("unknown Zstd command".into()),
    };
    Ok(match result {
        Ok(bytes) => crate::operations::encode(json!({"ok":true}), &bytes),
        Err(error) => crate::operations::encode(
            json!({"ok":false,"kind":format!("{:?}", error.kind()),"message":error.to_string()}),
            &[],
        ),
    })
}
fn decode(input: &[u8], limit: Option<u64>) -> io::Result<Vec<u8>> {
    let mut decoder = zstd::Decoder::new(input)?;
    let mut bytes = Vec::new();
    if let Some(limit) = limit {
        decoder.by_ref().take(limit).read_to_end(&mut bytes)?;
        let mut extra = [0];
        if decoder.read(&mut extra)? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Zstd output exceeds configured limit",
            ));
        }
    } else {
        decoder.read_to_end(&mut bytes)?;
    }
    Ok(bytes)
}
