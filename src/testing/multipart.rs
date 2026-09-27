use super::{TestError, error};
use std::sync::atomic::{AtomicU64, Ordering};

/// A binary part; filename and MIME type are optional.
pub struct Part {
    bytes: Vec<u8>,
    filename: Option<String>,
    mime: Option<String>,
}
impl Part {
    pub fn bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
            filename: None,
            mime: None,
        }
    }
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }
    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime = Some(mime.into());
        self
    }
}
/// Ordered multipart fields, preserving repeated names and exact file bytes.
#[derive(Default)]
pub struct MultipartForm {
    parts: Vec<(String, Part)>,
}
impl MultipartForm {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_text(self, name: impl Into<String>, text: impl Into<String>) -> Self {
        self.add_part(name, Part::bytes(text.into().into_bytes()))
    }
    pub fn add_part(mut self, name: impl Into<String>, part: Part) -> Self {
        self.parts.push((name.into(), part));
        self
    }
    pub(super) fn encode(self) -> Result<(String, Vec<u8>), TestError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let boundary = loop {
            let candidate = format!(
                "simple-server-test-{:016x}",
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            if self.parts.iter().all(|(_, p)| {
                !p.bytes
                    .windows(candidate.len())
                    .any(|b| b == candidate.as_bytes())
            }) {
                break candidate;
            }
        };
        let mut bytes = Vec::new();
        for (name, part) in self.parts {
            let quote = |s: &str| -> Result<String, TestError> {
                if s.contains(['\r', '\n', '\0']) {
                    return Err(error("invalid multipart metadata"));
                }
                Ok(s.replace('\\', "\\\\").replace('"', "\\\""))
            };
            bytes.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"",
                    quote(&name)?
                )
                .as_bytes(),
            );
            if let Some(filename) = part.filename {
                bytes.extend_from_slice(format!("; filename=\"{}\"", quote(&filename)?).as_bytes());
            }
            if let Some(mime) = part.mime {
                let _ = http::HeaderValue::from_str(&mime)?;
                bytes.extend_from_slice(format!("\r\nContent-Type: {mime}").as_bytes());
            }
            bytes.extend_from_slice(b"\r\n\r\n");
            bytes.extend_from_slice(&part.bytes);
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        Ok((format!("multipart/form-data; boundary={boundary}"), bytes))
    }
}
