//! SHA-256 in the native engine, with bounded buffering for incremental updates.
//!
//! No runtime is needed. Calls are synchronous and copy bytes across the ABI.
//! The `try_*` methods and [`std::io::Write`] return backend errors. Convenience
//! methods panic on an unavailable/incompatible engine, never return a substitute
//! digest. Dropping a hasher discards pending bytes and releases native state.
use serde_json::json;
use std::{
    fmt,
    io::{self, Write},
};
const KIND: u32 = 22;
const BUFFER: usize = 8192;

/// The 32 SHA-256 bytes, with lowercase hexadecimal formatting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Digest([u8; 32]);
impl AsRef<[u8]> for Digest {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl From<Digest> for [u8; 32] {
    fn from(digest: Digest) -> Self {
        digest.0
    }
}
impl fmt::LowerHex for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
fn call(header: serde_json::Value, bytes: &[u8]) -> io::Result<Vec<u8>> {
    let request = simple_server_sys::frame(&serde_json::to_vec(&header)?, bytes)?;
    simple_server_sys::resource_new(&request)
}
fn digest_response(response: &[u8]) -> io::Result<Digest> {
    let (_, body) = simple_server_sys::unframe(response)?;
    body.try_into()
        .map(Digest)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid native SHA-256 digest"))
}
/// Incremental SHA-256. Small updates are coalesced into 8 KiB chunks.
/// No complete-input buffer is retained; unfinished state is released on drop.
pub struct Sha256 {
    resource: simple_server_sys::Resource,
    pending: Vec<u8>,
    failed: bool,
}
impl Sha256 {
    /// Hash a byte buffer, returning errors from the native backend.
    pub fn try_digest(input: impl AsRef<[u8]>) -> io::Result<Digest> {
        digest_response(&call(json!({"op":"sha256_digest"}), input.as_ref())?)
    }
    /// Hash a byte buffer. Panics if the engine cannot perform SHA-256.
    pub fn digest(input: impl AsRef<[u8]>) -> Digest {
        Self::try_digest(input).expect("native SHA-256 backend")
    }
    /// Allocate independent native state, returning backend errors.
    pub fn try_new() -> io::Result<Self> {
        let response = call(json!({"op":"sha256_new"}), &[])?;
        let (header, _) = simple_server_sys::unframe(&response)?;
        let header: serde_json::Value = serde_json::from_slice(header)?;
        let id = header["id"].as_u64().filter(|id| *id != 0).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid native SHA-256 handle")
        })?;
        Ok(Self {
            resource: simple_server_sys::Resource::new(KIND, id),
            pending: Vec::with_capacity(BUFFER),
            failed: false,
        })
    }
    /// Allocate independent native state. Panics if the engine is incompatible.
    pub fn new() -> Self {
        Self::try_new().expect("native SHA-256 backend")
    }
    fn healthy(&self) -> io::Result<()> {
        if self.failed {
            Err(io::Error::other("native SHA-256 state failed"))
        } else {
            Ok(())
        }
    }
    fn send(&mut self, input: &[u8]) -> io::Result<()> {
        self.healthy()?;
        let result = call(
            json!({"op":"sha256_update", "id":self.resource.id()}),
            input,
        );
        if result.is_err() {
            self.failed = true;
        }
        result.map(|_| ())
    }
    /// Append bytes. A failed update permanently poisons this state so callers
    /// cannot finalize an ambiguous or partial digest after an ABI error.
    pub fn try_update(&mut self, input: impl AsRef<[u8]>) -> io::Result<()> {
        self.healthy()?;
        let mut input = input.as_ref();
        if !self.pending.is_empty() {
            let n = input.len().min(BUFFER - self.pending.len());
            self.pending.extend_from_slice(&input[..n]);
            input = &input[n..];
            if self.pending.len() == BUFFER {
                self.flush()?;
            }
        }
        if input.len() >= BUFFER {
            self.send(input)?;
        } else {
            self.pending.extend_from_slice(input);
        }
        Ok(())
    }
    /// Append bytes. Panics on backend failure; prefer `try_update` to recover.
    pub fn update(&mut self, input: impl AsRef<[u8]>) {
        self.try_update(input).expect("native SHA-256 backend");
    }
    /// Flush pending bytes and consume this state to return the final digest.
    pub fn try_finalize(mut self) -> io::Result<Digest> {
        self.flush()?;
        digest_response(&call(
            json!({"op":"sha256_finalize", "id":self.resource.id()}),
            &[],
        )?)
    }
    /// Consume this state. Panics on backend failure; never returns partial output.
    pub fn finalize(self) -> Digest {
        self.try_finalize().expect("native SHA-256 backend")
    }
}
impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}
impl Write for Sha256 {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.try_update(bytes)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.healthy()?;
        if !self.pending.is_empty() {
            let mut bytes = std::mem::take(&mut self.pending);
            let result = self.send(&bytes);
            bytes.clear();
            self.pending = bytes;
            result?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_update_cannot_be_retried_or_finalized() {
        let mut state = Sha256::new();
        call(
            json!({"op":"sha256_finalize", "id":state.resource.id()}),
            &[],
        )
        .unwrap();
        assert!(state.try_update(vec![42; BUFFER]).is_err());
        assert!(state.try_update([]).is_err());
        assert!(state.flush().is_err());
        assert!(state.try_finalize().is_err());
    }
}
