//! Byte-buffer Zstd codecs executed synchronously in the native engine.
//!
//! No runtime is required. Use a blocking worker for large inputs from async
//! handlers. These functions buffer the complete result; they are not streaming
//! readers/writers. Binary input/output uses the existing framed ABI, not JSON
//! byte arrays. The engine must support the Zstd commands.
use serde_json::json;
use std::io;

/// Compress a buffer at the selected Zstd level (zero selects its default).
pub fn encode_all(input: &[u8], level: i32) -> io::Result<Vec<u8>> {
    command(json!({"op":"zstd_encode", "level":level}), input)
}

/// Decode all concatenated frames without an application-imposed output limit.
/// Prefer [`decode_all_limited`] for inputs whose expanded size is untrusted.
pub fn decode_all(input: &[u8]) -> io::Result<Vec<u8>> {
    command(json!({"op":"zstd_decode"}), input)
}

/// Decode all frames, rejecting output larger than `max_bytes` without returning
/// a partial result. An exactly-sized output is accepted, including zero bytes.
/// This bounds output length, not the decoder's window allocation or CPU time.
pub fn decode_all_limited(input: &[u8], max_bytes: usize) -> io::Result<Vec<u8>> {
    command(json!({"op":"zstd_decode", "max_bytes":max_bytes}), input)
}

fn command(header: serde_json::Value, input: &[u8]) -> io::Result<Vec<u8>> {
    let request = simple_server_sys::frame(&serde_json::to_vec(&header)?, input)?;
    let mut output = simple_server_sys::resource_new(&request)?;
    let (header, body) = simple_server_sys::unframe(&output)?;
    let header: serde_json::Value = serde_json::from_slice(header)?;
    if header["ok"] != true {
        let kind = match header["kind"].as_str() {
            Some("InvalidInput") => io::ErrorKind::InvalidInput,
            Some("InvalidData") => io::ErrorKind::InvalidData,
            Some("UnexpectedEof") => io::ErrorKind::UnexpectedEof,
            _ => io::ErrorKind::Other,
        };
        return Err(io::Error::new(
            kind,
            header["message"]
                .as_str()
                .unwrap_or("Zstd codec failed")
                .to_owned(),
        ));
    }
    // Keep the host result allocation rather than making another full-sized copy.
    let offset = output.len() - body.len();
    let length = body.len();
    output.copy_within(offset.., 0);
    output.truncate(length);
    Ok(output)
}
