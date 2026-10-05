use serde_json::Value;
use std::io;

pub(crate) async fn command(header: Value, payload: &[u8]) -> io::Result<(Value, Vec<u8>)> {
    let header = serde_json::to_vec(&header)?;
    let output = simple_server_sys::Operation::command(&header, payload)?.await?;
    decode(&output)
}

pub(crate) fn decode(output: &[u8]) -> io::Result<(Value, Vec<u8>)> {
    let (header, payload) = simple_server_sys::unframe(output)?;
    Ok((serde_json::from_slice(header)?, payload.to_vec()))
}
