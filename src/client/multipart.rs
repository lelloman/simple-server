//! Streaming outbound multipart bodies; encoding and boundaries live in the engine.
use super::Error;
use serde_json::{Value, json};
use simple_server_sys::Callback;
use std::{
    io::Read,
    sync::{Arc, Mutex},
};
#[derive(Default)]
pub struct Form {
    parts: Vec<(String, Part)>,
}
impl Form {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn part(mut self, name: impl Into<String>, part: Part) -> Self {
        self.parts.push((name.into(), part));
        self
    }
    pub fn text(self, name: impl Into<String>, text: impl Into<String>) -> Self {
        self.part(name, Part::bytes(text.into().into_bytes()))
    }
    pub(super) fn encode(self) -> Result<(Value, Vec<Callback>), Error> {
        let mut metadata = Vec::new();
        let mut callbacks = Vec::new();
        for (name, part) in self.parts {
            let reader = Arc::new(Mutex::new(part.reader));
            let callback = Callback::new(move |_| {
                let reader = reader.clone();
                async move {
                    let result = crate::runtime::spawn_blocking(move || {
                        let mut buffer = vec![0; 64 * 1024];
                        let length = reader.lock().unwrap().read(&mut buffer)?;
                        buffer.truncate(length);
                        Ok::<_, std::io::Error>(buffer)
                    })
                    .await
                    .map_err(|e| std::io::Error::other(e.to_string()))
                    .and_then(|v| v);
                    let (header, body) = match result {
                        Ok(body) => (json!({"ok":true,"eof":body.is_empty()}), body),
                        Err(e) => (json!({"ok":false,"message":e.to_string()}), Vec::new()),
                    };
                    simple_server_sys::frame(
                        &serde_json::to_vec(&header).expect("multipart header"),
                        &body,
                    )
                    .expect("multipart frame")
                }
            })
            .map_err(Error::local)?;
            metadata.push(json!({"name":name,"file_name":part.file_name,"mime":part.mime,"length":part.length,"callback":callback.id()}));
            callbacks.push(callback);
        }
        Ok((json!(metadata), callbacks))
    }
}
pub struct Part {
    reader: Box<dyn Read + Send>,
    length: Option<u64>,
    file_name: Option<String>,
    mime: Option<String>,
}
impl Part {
    pub fn bytes(bytes: impl Into<Vec<u8>>) -> Self {
        let bytes = bytes.into();
        let length = bytes.len() as u64;
        Self::reader_with_length(std::io::Cursor::new(bytes), length)
    }
    pub fn reader_with_length(reader: impl Read + Send + 'static, length: u64) -> Self {
        Self {
            reader: Box::new(reader),
            length: Some(length),
            file_name: None,
            mime: None,
        }
    }
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }
    pub fn mime_str(mut self, mime: &str) -> Result<Self, Error> {
        mime.parse::<mime::Mime>().map_err(Error::local)?;
        self.mime = Some(mime.to_owned());
        Ok(self)
    }
}
