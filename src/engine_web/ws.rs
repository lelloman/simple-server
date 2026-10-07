//! Native WebSocket upgrade and duplex messages. Application tasks own drain policy.
use super::{Bytes, HeaderValue, RejectionResponse, Response, wire};
use futures_util::{Sink, SinkExt, Stream, StreamExt, stream::FusedStream};
use serde_json::{Value, json};
use simple_server_sys::{Callback, Operation, Resource};
use std::{
    borrow::Cow,
    fmt,
    future::Future,
    io,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};
#[derive(Debug)]
pub struct WebSocketError(io::Error);
impl WebSocketError {
    fn new(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(io::Error::other(e))
    }
}
impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for WebSocketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
impl From<io::Error> for WebSocketError {
    fn from(e: io::Error) -> Self {
        Self(e)
    }
}
/// Validated UTF-8 text with cheaply cloned byte storage.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Utf8Bytes(Bytes);
impl Utf8Bytes {
    pub const fn from_static(value: &'static str) -> Self {
        Self(Bytes::from_static(value.as_bytes()))
    }
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("validated UTF-8")
    }
}
impl std::ops::Deref for Utf8Bytes {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<str> for Utf8Bytes {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for Utf8Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl From<String> for Utf8Bytes {
    fn from(value: String) -> Self {
        Self(Bytes::copy_from_slice(value.as_bytes()))
    }
}
impl From<&str> for Utf8Bytes {
    fn from(value: &str) -> Self {
        Self(Bytes::copy_from_slice(value.as_bytes()))
    }
}
impl From<&String> for Utf8Bytes {
    fn from(value: &String) -> Self {
        Self(Bytes::copy_from_slice(value.as_bytes()))
    }
}
impl TryFrom<Bytes> for Utf8Bytes {
    type Error = std::str::Utf8Error;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        std::str::from_utf8(&value)?;
        Ok(Self(value))
    }
}
impl TryFrom<Vec<u8>> for Utf8Bytes {
    type Error = std::str::Utf8Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(Bytes::from(value))
    }
}
impl From<Utf8Bytes> for Bytes {
    fn from(value: Utf8Bytes) -> Self {
        value.0
    }
}

pub type CloseCode = u16;
/// Close status and UTF-8 reason. Codes and reasons are validated by the transport.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloseFrame {
    pub code: CloseCode,
    pub reason: Utf8Bytes,
}
/// Complete messages; continuation frames are reassembled by the transport.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Message {
    Text(Utf8Bytes),
    Binary(Bytes),
    Ping(Bytes),
    Pong(Bytes),
    Close(Option<CloseFrame>),
}
impl Message {
    pub fn text(value: impl Into<Utf8Bytes>) -> Self {
        Self::Text(value.into())
    }
    pub fn binary(value: impl Into<Bytes>) -> Self {
        Self::Binary(value.into())
    }
    pub fn into_data(self) -> Bytes {
        match self {
            Self::Text(v) => v.into(),
            Self::Binary(v) | Self::Ping(v) | Self::Pong(v) => v,
            Self::Close(Some(v)) => v.reason.into(),
            Self::Close(None) => Bytes::new(),
        }
    }
    pub fn into_text(self) -> Result<Utf8Bytes, WebSocketError> {
        Utf8Bytes::try_from(self.into_data()).map_err(WebSocketError::new)
    }
    pub fn to_text(&self) -> Result<&str, WebSocketError> {
        match self {
            Self::Text(v) => Ok(v.as_str()),
            Self::Binary(v) | Self::Ping(v) | Self::Pong(v) => {
                std::str::from_utf8(v).map_err(WebSocketError::new)
            }
            Self::Close(Some(v)) => Ok(v.reason.as_str()),
            Self::Close(None) => Ok(""),
        }
    }
}

pub struct WebSocket {
    resource: Resource,
    protocol: Option<HeaderValue>,
    read: Option<Operation>,
    write: Option<Operation>,
    control: Option<Operation>,
    control_kind: Option<&'static str>,
    ended: bool,
}
impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSocket")
            .field("protocol", &self.protocol)
            .finish_non_exhaustive()
    }
}
fn operation(resource: &Resource, op: &str) -> io::Result<Operation> {
    Operation::new(&wire::encode(json!({"op":op,"socket":resource.id()}), &[])?)
}
fn poll_operation(
    pending: &mut Option<Operation>,
    cx: &mut Context<'_>,
) -> Poll<Result<Vec<u8>, WebSocketError>> {
    let bytes = std::task::ready!(Pin::new(pending.as_mut().expect("pending operation")).poll(cx));
    *pending = None;
    let bytes = bytes?;
    let (header, _) = wire::decode(&bytes)?;
    wire::check(&header)?;
    Poll::Ready(Ok(bytes))
}
impl WebSocket {
    pub async fn recv(&mut self) -> Option<Result<Message, WebSocketError>> {
        self.next().await
    }
    pub async fn send(&mut self, message: Message) -> Result<(), WebSocketError> {
        SinkExt::send(self, message).await
    }
    pub async fn close(&mut self) -> Result<(), WebSocketError> {
        SinkExt::close(self).await
    }
    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }
    fn poll_control(
        &mut self,
        cx: &mut Context<'_>,
        op: &'static str,
    ) -> Poll<Result<(), WebSocketError>> {
        if self.write.is_some() {
            std::task::ready!(poll_operation(&mut self.write, cx))?;
        }
        if self.control.is_some() {
            std::task::ready!(poll_operation(&mut self.control, cx))?;
            if self.control_kind.take() == Some(op) {
                return Poll::Ready(Ok(()));
            }
        }
        self.control = Some(operation(&self.resource, op)?);
        self.control_kind = Some(op);
        let result = std::task::ready!(poll_operation(&mut self.control, cx));
        self.control_kind = None;
        Poll::Ready(result.map(|_| ()))
    }
}
impl Stream for WebSocket {
    type Item = Result<Message, WebSocketError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let s = self.get_mut();
        if s.ended {
            return Poll::Ready(None);
        }
        let result = (|| -> Poll<Result<Option<Message>, WebSocketError>> {
            if s.read.is_none() {
                s.read = Some(operation(&s.resource, "ws_recv")?);
            }
            let bytes = std::task::ready!(poll_operation(&mut s.read, cx))?;
            let (header, body) = wire::decode(&bytes)?;
            let data = Bytes::copy_from_slice(body);
            let msg = match header["kind"].as_str() {
                Some("end") => return Poll::Ready(Ok(None)),
                Some("text") => {
                    Message::Text(Utf8Bytes::try_from(data).map_err(WebSocketError::new)?)
                }
                Some("binary") => Message::Binary(data),
                Some("ping") => Message::Ping(data),
                Some("pong") => Message::Pong(data),
                Some("close") => Message::Close(
                    header["code"]
                        .as_u64()
                        .map(|code| -> Result<CloseFrame, WebSocketError> {
                            Ok(CloseFrame {
                                code: u16::try_from(code).map_err(WebSocketError::new)?,
                                reason: Utf8Bytes::try_from(data).map_err(WebSocketError::new)?,
                            })
                        })
                        .transpose()?,
                ),
                _ => {
                    return Poll::Ready(Err(WebSocketError::new(io::Error::other(
                        "invalid native WebSocket message",
                    ))));
                }
            };
            Poll::Ready(Ok(Some(msg)))
        })();
        match result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(None)) => {
                s.ended = true;
                Poll::Ready(None)
            }
            Poll::Ready(Ok(Some(m))) => Poll::Ready(Some(Ok(m))),
            Poll::Ready(Err(e)) => {
                s.ended = true;
                Poll::Ready(Some(Err(e)))
            }
        }
    }
}
impl FusedStream for WebSocket {
    fn is_terminated(&self) -> bool {
        self.ended
    }
}
impl Sink<Message> for WebSocket {
    type Error = WebSocketError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let s = self.get_mut();
        if s.control.is_some() {
            std::task::ready!(poll_operation(&mut s.control, cx))?;
            s.control_kind = None;
        }
        if s.write.is_some() {
            std::task::ready!(poll_operation(&mut s.write, cx))?;
        }
        Poll::Ready(Ok(()))
    }
    fn start_send(self: Pin<&mut Self>, message: Message) -> Result<(), Self::Error> {
        let s = self.get_mut();
        if s.write.is_some() {
            return Err(WebSocketError::new(io::Error::other(
                "WebSocket sink is not ready",
            )));
        }
        let (kind, code) = match &message {
            Message::Text(_) => ("text", None),
            Message::Binary(_) => ("binary", None),
            Message::Ping(_) => ("ping", None),
            Message::Pong(_) => ("pong", None),
            Message::Close(c) => ("close", c.as_ref().map(|c| c.code)),
        };
        s.write = Some(Operation::new(&wire::encode(
            json!({"op":"ws_send","socket":s.resource.id(),"kind":kind,"code":code}),
            &message.into_data(),
        )?)?);
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().poll_control(cx, "ws_flush")
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().poll_control(cx, "ws_close")
    }
}
#[must_use]
pub struct WebSocketUpgrade {
    resource: Resource,
    config: Value,
    requested: Vec<HeaderValue>,
    selected: Option<HeaderValue>,
    failed: Option<Box<dyn FnOnce(WebSocketError) + Send>>,
}
impl<S: Send + Sync> super::FromRequestParts<S> for WebSocketUpgrade {
    type Rejection = RejectionResponse;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let result: Result<Self, (http::StatusCode, String)> = async {
            let native = parts
                .extensions
                .get::<super::continuation::NativeExtensions>()
                .ok_or_else(|| {
                    (
                        http::StatusCode::UPGRADE_REQUIRED,
                        "WebSocket upgrade requires a native HTTP connection".into(),
                    )
                })?;
            let resource =
                wire::resource(24, json!({"op": "ws_upgrade_slot"})).map_err(internal)?;
            let request = wire::encode(
                json!({
                    "op": "ws_extract",
                    "upgrade": resource.id(),
                    "extensions": native.0.id(),
                    "method": parts.method.as_str(),
                    "version": format!("{:?}", parts.version),
                    "headers": wire::headers(&parts.headers),
                }),
                &[],
            )
            .map_err(internal)?;
            let bytes = Operation::new(&request)
                .map_err(internal)?
                .await
                .map_err(internal)?;
            let (header, _) = wire::decode(&bytes).map_err(internal)?;
            if header["ok"] != true {
                let status = header["status"]
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .and_then(|v| http::StatusCode::from_u16(v).ok())
                    .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
                let message = header["message"]
                    .as_str()
                    .unwrap_or("WebSocket upgrade failed")
                    .to_owned();
                return Err((status, message));
            }
            let requested = parts
                .headers
                .get_all(http::header::SEC_WEBSOCKET_PROTOCOL)
                .iter()
                .cloned()
                .collect();
            Ok(Self {
                resource,
                config: json!({}),
                requested,
                selected: None,
                failed: None,
            })
        }
        .await;
        result.map_err(|(status, message)| {
            let mut r = RejectionResponse::new(message.into_bytes());
            *r.status_mut() = status;
            r.headers_mut().insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            r
        })
    }
}
fn internal(e: io::Error) -> (http::StatusCode, String) {
    (http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
impl WebSocketUpgrade {
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.config["read_buffer_size"] = json!(size);
        self
    }
    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.config["write_buffer_size"] = json!(size);
        self
    }
    pub fn max_write_buffer_size(mut self, size: usize) -> Self {
        self.config["max_write_buffer_size"] = json!(size);
        self
    }
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config["max_message_size"] = json!(size);
        self
    }
    pub fn max_frame_size(mut self, size: usize) -> Self {
        self.config["max_frame_size"] = json!(size);
        self
    }
    pub fn accept_unmasked_frames(mut self, accept: bool) -> Self {
        self.config["accept_unmasked_frames"] = json!(accept);
        self
    }
    pub fn protocols<I>(mut self, protocols: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'static, str>>,
    {
        self.selected = protocols
            .into_iter()
            .map(Into::into)
            .find(|candidate| {
                self.requested
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .flat_map(|v| v.split(','))
                    .any(|v| v.trim() == candidate.as_ref())
            })
            .and_then(|v| HeaderValue::from_str(&v).ok());
        self
    }
    pub fn requested_protocols(&self) -> impl Iterator<Item = &HeaderValue> {
        self.requested.iter()
    }
    pub fn selected_protocol(&self) -> Option<&HeaderValue> {
        self.selected.as_ref()
    }
    pub fn set_selected_protocol(&mut self, protocol: HeaderValue) {
        self.selected = Some(protocol)
    }
    pub fn on_failed_upgrade(
        mut self,
        callback: impl FnOnce(WebSocketError) + Send + 'static,
    ) -> Self {
        self.failed = Some(Box::new(callback));
        self
    }
    #[must_use = "return this response to accept the upgrade"]
    pub fn on_upgrade<C, F>(mut self, callback: C) -> Response
    where
        C: FnOnce(WebSocket) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let selected = self.selected.clone();
        let host = super::host_runtime::HostRuntime::capture();
        let callbacks = host.own(Mutex::new(Some((callback, self.failed))));
        let result = (|| -> io::Result<Response> {
            let callback = Callback::new(move |bytes| {
                let callbacks = callbacks.lock().unwrap().take();
                let selected = selected.clone();
                host.scope(async move {
                    if let Some((callback, failed)) = callbacks {
                        let result = (|| -> io::Result<Resource> {
                            let (header, _) = wire::decode(&bytes)?;
                            wire::check(&header)?;
                            wire::resource(
                                25,
                                json!({"op":"ws_socket_clone","socket":header["socket"]}),
                            )
                        })();
                        match result {
                            Ok(resource) => {
                                callback(WebSocket {
                                    resource,
                                    protocol: selected,
                                    read: None,
                                    write: None,
                                    control: None,
                                    control_kind: None,
                                    ended: false,
                                })
                                .await
                            }
                            Err(e) => {
                                if let Some(failed) = failed {
                                    failed(e.into())
                                }
                            }
                        }
                    }
                    Vec::new()
                })
            })?;
            self.config["op"] = json!("ws_accept");
            self.config["upgrade"] = json!(self.resource.id());
            self.config["callback"] = json!(callback.id());
            if let Some(protocol) = self.selected {
                self.config["selected_protocol"] =
                    json!(protocol.to_str().map_err(io::Error::other)?);
            }
            let bytes = simple_server_sys::resource_new(&wire::encode(self.config, &[])?)?;
            let (header, _) = wire::decode(&bytes)?;
            wire::check(&header)?;
            let mut response = Response::new(super::Body::empty());
            *response.status_mut() = http::StatusCode::from_u16(
                header["status"]
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .ok_or_else(|| io::Error::other("missing upgrade status"))?,
            )
            .map_err(io::Error::other)?;
            *response.headers_mut() = wire::read_headers(&header["headers"])?;
            Ok(response)
        })();
        result.unwrap_or_else(|_| {
            let mut r = Response::new(super::Body::from("WebSocket upgrade failed"));
            *r.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
            r
        })
    }
}
/// Standard close statuses. STATUS and ABNORMAL are observations, never wire codes.
pub mod close_code {
    pub const NORMAL: u16 = 1000;
    pub const AWAY: u16 = 1001;
    pub const PROTOCOL: u16 = 1002;
    pub const UNSUPPORTED: u16 = 1003;
    pub const STATUS: u16 = 1005;
    pub const ABNORMAL: u16 = 1006;
    pub const INVALID: u16 = 1007;
    pub const POLICY: u16 = 1008;
    pub const SIZE: u16 = 1009;
    pub const EXTENSION: u16 = 1010;
    pub const ERROR: u16 = 1011;
    pub const RESTART: u16 = 1012;
    pub const AGAIN: u16 = 1013;
}
