//! Parser and field resources stay in the engine. Field slots are allocated
//! synchronously before async parsing, so cancelled operations cannot orphan IDs.
// Adapted from axum 0.8.9, src/extract/multipart.rs.
// Rejection classification adapted for the multipart wire protocol.
// Upstream license follows.
// Copyright (c) 2019 axum Contributors
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use crate::{
    operations::{FutureBytes, encode, id},
    server,
};
use axum::body::Body;
use http_body_util::Limited;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex, OnceLock},
};

pub const PARSER: u32 = 11;
pub const FIELD: u32 = 12;
type Parser = Arc<tokio::sync::Mutex<multer::Multipart<'static>>>;
type Field = Arc<tokio::sync::Mutex<Option<multer::Field<'static>>>>;
static PARSERS: OnceLock<Mutex<HashMap<u64, Parser>>> = OnceLock::new();
static FIELDS: OnceLock<Mutex<HashMap<u64, Field>>> = OnceLock::new();
fn parsers() -> &'static Mutex<HashMap<u64, Parser>> {
    PARSERS.get_or_init(Default::default)
}
fn fields() -> &'static Mutex<HashMap<u64, Field>> {
    FIELDS.get_or_init(Default::default)
}
fn number(command: &Value, key: &str) -> Result<u64, String> {
    command[key]
        .as_u64()
        .ok_or_else(|| format!("missing {key}"))
}
pub fn release(kind: u32, id: u64) {
    // Callback destructors can reenter the engine. Never drop under map locks.
    match kind {
        PARSER => {
            let value = parsers().lock().unwrap().remove(&id);
            drop(value);
        }
        FIELD => {
            let value = fields().lock().unwrap().remove(&id);
            drop(value);
        }
        _ => (),
    }
}
pub fn resource_new(command: &Value) -> Result<Vec<u8>, String> {
    let id = id();
    if command["op"] == "multipart_field_slot" {
        fields()
            .lock()
            .unwrap()
            .insert(id, Arc::new(tokio::sync::Mutex::new(None)));
    } else {
        let boundary = command["content_type"]
            .as_str()
            .and_then(|text| multer::parse_boundary(text).ok());
        let Some(boundary) = boundary else {
            return Ok(encode(
                json!({"ok":false,"status":400,"text":"Invalid `boundary` for `multipart/form-data` request"}),
                &[],
            ));
        };
        let limit = usize::try_from(number(command, "limit")?).map_err(|e| e.to_string())?;
        let body = server::callback_body(number(command, "body")?)?;
        let body = Body::new(Limited::new(body, limit));
        let parser = multer::Multipart::new(body.into_data_stream(), boundary);
        parsers()
            .lock()
            .unwrap()
            .insert(id, Arc::new(tokio::sync::Mutex::new(parser)));
    }
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
fn failure(error: multer::Error) -> Vec<u8> {
    // Keep the source extractor's status and diagnostics without exposing its
    // Rust error type across the ABI.
    let status = match &error {
        multer::Error::UnknownField { .. }
        | multer::Error::IncompleteFieldData { .. }
        | multer::Error::IncompleteHeaders
        | multer::Error::ReadHeaderFailed(..)
        | multer::Error::DecodeHeaderName { .. }
        | multer::Error::DecodeContentType(..)
        | multer::Error::NoBoundary
        | multer::Error::DecodeHeaderValue { .. }
        | multer::Error::NoMultipart
        | multer::Error::IncompleteStream => 400,
        multer::Error::FieldSizeExceeded { .. } | multer::Error::StreamSizeExceeded { .. } => 413,
        multer::Error::StreamReadFailed(cause) => {
            let mut cause: &(dyn Error + 'static) = &**cause;
            loop {
                if cause.is::<http_body_util::LengthLimitError>() {
                    break 413;
                }
                match cause.source() {
                    Some(next) => cause = next,
                    None => break 500,
                }
            }
        }
        _ => 500,
    };
    encode(
        json!({"ok":false,"status":status,"text":error.to_string(),"message":"Error parsing `multipart/form-data` request"}),
        &[],
    )
}
pub fn operation(command: Value) -> Result<FutureBytes, String> {
    let field = fields()
        .lock()
        .unwrap()
        .get(&number(&command, "field")?)
        .cloned()
        .ok_or("multipart field released")?;
    match command["op"].as_str() {
        Some("multipart_next") => {
            let parser = parsers()
                .lock()
                .unwrap()
                .get(&number(&command, "parser")?)
                .cloned()
                .ok_or("multipart parser released")?;
            Ok(Box::pin(async move {
                let mut slot = field.lock().await;
                if slot.is_some() {
                    return failure(multer::Error::LockFailure);
                }
                match parser.lock().await.next_field().await {
                    Ok(Some(next)) => {
                        let output = encode(
                            json!({"ok":true,"end":false,"name":next.name(),"file_name":next.file_name(),"content_type":next.content_type().map(|v|v.as_ref()),"headers":server::headers_to_wire(next.headers())}),
                            &[],
                        );
                        *slot = Some(next);
                        output
                    }
                    Ok(None) => encode(json!({"ok":true,"end":true}), &[]),
                    Err(error) => failure(error),
                }
            }))
        }
        Some("multipart_chunk") => Ok(Box::pin(async move {
            let mut slot = field.lock().await;
            let Some(field) = slot.as_mut() else {
                return failure(multer::Error::LockFailure);
            };
            match field.chunk().await {
                Ok(Some(bytes)) => encode(json!({"ok":true,"end":false}), &bytes),
                Ok(None) => encode(json!({"ok":true,"end":true}), &[]),
                Err(error) => failure(error),
            }
        })),
        Some("multipart_text") => Ok(Box::pin(async move {
            let Some(field) = field.lock().await.take() else {
                return failure(multer::Error::LockFailure);
            };
            match field.text().await {
                Ok(text) => encode(json!({"ok":true}), text.as_bytes()),
                Err(error) => failure(error),
            }
        })),
        _ => Err("unknown multipart operation".into()),
    }
}
