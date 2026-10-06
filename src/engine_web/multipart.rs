//! Streaming multipart parsing in the shared engine. No parser dependency is
//! compiled by consumers. Requires an engine with multipart resource support.
//! The default 2 MiB request limit can be overridden by a `BodyLimit` extension.
//! Application-specific file limits, storage, and filename validation remain
//! caller-owned. Only `bytes` and `text` collect a complete field.
use super::{
    BodyLimit, Bytes, FromRequest, HeaderMap, IntoRejectionResponse, IntoResponse,
    RejectionResponse, Request, Response, StatusCode, body, wire,
};
use futures_util::{Stream, StreamExt};
use serde_json::json;
use simple_server_sys::{Operation, Resource};
use std::{
    error::Error,
    fmt,
    future::Future,
    io,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

/// Parser/body failure. The wire preserves status and diagnostics, while the
/// source error is a host-owned diagnostic rather than a native Rust error.
#[derive(Debug)]
pub struct MultipartError {
    status: StatusCode,
    text: String,
    message: String,
    source: io::Error,
}
impl MultipartError {
    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn body_text(&self) -> String {
        self.text.clone()
    }
    fn borrowed(mut self) -> Self {
        if self.status == StatusCode::PAYLOAD_TOO_LARGE {
            self.text = "Request payload is too large".to_owned();
        }
        self
    }
    fn transport(error: io::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            text: error.to_string(),
            message: error.to_string(),
            source: error,
        }
    }
    fn check(header: &serde_json::Value) -> Result<(), Self> {
        if header["ok"] == true {
            return Ok(());
        }
        let text = header["text"]
            .as_str()
            .unwrap_or("engine multipart operation failed")
            .to_owned();
        Err(Self {
            status: header["status"]
                .as_u64()
                .and_then(|v| u16::try_from(v).ok())
                .and_then(|v| StatusCode::from_u16(v).ok())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            message: header["message"].as_str().unwrap_or(&text).to_owned(),
            source: io::Error::other(text.clone()),
            text,
        })
    }
}
impl fmt::Display for MultipartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl Error for MultipartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
impl IntoRejectionResponse for MultipartError {
    fn into_rejection_response(self) -> RejectionResponse {
        super::extract::rejection(self.status, self.text)
    }
}
impl IntoResponse for MultipartError {
    fn into_response(self) -> Response {
        self.into_rejection_response().into_response()
    }
}

/// Reader with statically exclusive borrowed fields.
/// ```compile_fail
/// use simple_server::engine_web::multipart::Multipart;
/// async fn invalid(mut upload: Multipart) {
///     let first = upload.next_field().await.unwrap();
///     let second = upload.next_field().await.unwrap();
///     drop(first);
/// }
/// ```
pub struct Multipart(OwnedMultipart);
/// A field which borrows its reader until dropped or consumed.
pub struct Field<'a> {
    inner: OwnedField,
    _reader: PhantomData<&'a mut Multipart>,
}
/// Reader with runtime field exclusivity. Drop or consume a field before asking
/// for the next one. Owned fields may outlive the reader.
pub struct OwnedMultipart {
    resource: Resource,
}
/// A lazily streamed field with host-owned metadata and a native parser handle.
pub struct OwnedField {
    resource: Resource,
    name: Option<String>,
    file_name: Option<String>,
    content_type: Option<String>,
    headers: HeaderMap,
    pending: Option<Operation>,
    ended: bool,
}
impl<S: Sync> FromRequest<S> for OwnedMultipart {
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, _: &S) -> Result<Self, Self::Rejection> {
        fn create(request: Request) -> Result<OwnedMultipart, MultipartError> {
            let limit = request
                .extensions()
                .get::<BodyLimit>()
                .map_or(2 * 1024 * 1024, |v| v.0);
            let content_type = request
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let callback = body::export(request.into_body()).map_err(MultipartError::transport)?;
            let command = wire::encode(json!({"op":"multipart_new","body":callback.id(),"content_type":content_type,"limit":limit}), &[]).map_err(MultipartError::transport)?;
            let output =
                simple_server_sys::resource_new(&command).map_err(MultipartError::transport)?;
            let (header, _) = wire::decode(&output).map_err(MultipartError::transport)?;
            MultipartError::check(&header)?;
            Ok(OwnedMultipart {
                resource: Resource::new(11, wire::id(&header).map_err(MultipartError::transport)?),
            })
        }
        create(request).map_err(IntoRejectionResponse::into_rejection_response)
    }
}
impl<S: Sync> FromRequest<S> for Multipart {
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        OwnedMultipart::from_request(request, state).await.map(Self)
    }
}
impl Multipart {
    pub async fn next_field(&mut self) -> Result<Option<Field<'_>>, MultipartError> {
        self.0
            .next_field()
            .await
            .map_err(MultipartError::borrowed)
            .map(|field| {
                field.map(|inner| Field {
                    inner,
                    _reader: PhantomData,
                })
            })
    }
}
impl OwnedMultipart {
    pub async fn next_field(&mut self) -> Result<Option<OwnedField>, MultipartError> {
        // Allocate before awaiting: cancellation always drops this registration.
        let resource = wire::resource(12, json!({"op":"multipart_field_slot"}))
            .map_err(MultipartError::transport)?;
        let output = operation(
            json!({"op":"multipart_next","parser":self.resource.id(),"field":resource.id()}),
        )?
        .await
        .map_err(MultipartError::transport)?;
        let (header, _) = wire::decode(&output).map_err(MultipartError::transport)?;
        MultipartError::check(&header)?;
        if header["end"] == true {
            return Ok(None);
        }
        Ok(Some(OwnedField {
            resource,
            name: header["name"].as_str().map(str::to_owned),
            file_name: header["file_name"].as_str().map(str::to_owned),
            content_type: header["content_type"].as_str().map(str::to_owned),
            headers: wire::read_headers(&header["headers"]).map_err(MultipartError::transport)?,
            pending: None,
            ended: false,
        }))
    }
}
fn operation(command: serde_json::Value) -> Result<Operation, MultipartError> {
    wire::encode(command, &[])
        .and_then(|bytes| Operation::new(&bytes))
        .map_err(MultipartError::transport)
}
impl OwnedField {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub async fn chunk(&mut self) -> Result<Option<Bytes>, MultipartError> {
        self.next().await.transpose()
    }
    pub async fn bytes(mut self) -> Result<Bytes, MultipartError> {
        let mut bytes = Vec::new();
        while let Some(chunk) = self.chunk().await? {
            bytes.extend_from_slice(&chunk);
        }
        Ok(Bytes::from(bytes))
    }
    /// Collect and decode using the parser's content-type charset behavior.
    pub async fn text(mut self) -> Result<String, MultipartError> {
        // Cancel any uncompleted chunk poll before transferring field ownership.
        self.pending = None;
        let output = operation(json!({"op":"multipart_text","field":self.resource.id()}))?
            .await
            .map_err(MultipartError::transport)?;
        let (header, bytes) = wire::decode(&output).map_err(MultipartError::transport)?;
        MultipartError::check(&header)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| MultipartError::transport(io::Error::other(e)))
    }
}
impl Stream for OwnedField {
    type Item = Result<Bytes, MultipartError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.ended {
            return Poll::Ready(None);
        }
        if self.pending.is_none() {
            match operation(json!({"op":"multipart_chunk","field":self.resource.id()})) {
                Ok(op) => self.pending = Some(op),
                Err(error) => {
                    self.ended = true;
                    return Poll::Ready(Some(Err(error)));
                }
            }
        }
        let output = match Pin::new(self.pending.as_mut().unwrap()).poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(output) => output,
        };
        self.pending = None;
        let result = output.map_err(MultipartError::transport).and_then(|bytes| {
            let (header, data) = wire::decode(&bytes).map_err(MultipartError::transport)?;
            MultipartError::check(&header)?;
            Ok(if header["end"] == true {
                None
            } else {
                Some(Bytes::copy_from_slice(data))
            })
        });
        self.ended = !matches!(&result, Ok(Some(_)));
        Poll::Ready(result.transpose())
    }
}
impl Field<'_> {
    pub fn name(&self) -> Option<&str> {
        self.inner.name()
    }
    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name()
    }
    pub fn content_type(&self) -> Option<&str> {
        self.inner.content_type()
    }
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }
    pub async fn chunk(&mut self) -> Result<Option<Bytes>, MultipartError> {
        self.inner.chunk().await.map_err(MultipartError::borrowed)
    }
    pub async fn bytes(self) -> Result<Bytes, MultipartError> {
        self.inner.bytes().await.map_err(MultipartError::borrowed)
    }
    pub async fn text(self) -> Result<String, MultipartError> {
        self.inner.text().await.map_err(MultipartError::borrowed)
    }
}
impl Stream for Field<'_> {
    type Item = Result<Bytes, MultipartError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner)
            .poll_next(cx)
            .map(|item| item.map(|result| result.map_err(MultipartError::borrowed)))
    }
}
