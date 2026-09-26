//! Owned WebSocket upgrade, streaming and frame contracts (`web` + `ws`).
//!
//! Ping replies and close-handshake processing are handled by the transport.
//! Keep polling/flushing the socket to drive them. Authentication, heartbeat
//! policy and tracking upgraded connections during shutdown belong to callers.
use super::{Bytes, HeaderValue, RejectionResponse, Response};
use axum::extract::ws as backend;
use futures_util::{Sink, SinkExt, Stream, StreamExt, stream::FusedStream};
use std::{
    borrow::Cow,
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A transport, upgrade or text-decoding failure with an inspectable source.
#[derive(Debug)]
pub struct WebSocketError(Box<dyn std::error::Error + Send + Sync>);
impl WebSocketError {
    fn new(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(error))
    }
}
impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for WebSocketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// Validated UTF-8 text with cheaply cloned byte storage.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Utf8Bytes(backend::Utf8Bytes);
impl Utf8Bytes {
    pub const fn from_static(value: &'static str) -> Self {
        Self(backend::Utf8Bytes::from_static(value))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
        Self(value.into())
    }
}
impl From<&str> for Utf8Bytes {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}
impl From<&String> for Utf8Bytes {
    fn from(value: &String) -> Self {
        Self(value.into())
    }
}
impl TryFrom<Bytes> for Utf8Bytes {
    type Error = std::str::Utf8Error;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        value.try_into().map(Self)
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
        value.0.into()
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
    fn from_backend(value: backend::Message) -> Self {
        match value {
            backend::Message::Text(v) => Self::Text(Utf8Bytes(v)),
            backend::Message::Binary(v) => Self::Binary(v),
            backend::Message::Ping(v) => Self::Ping(v),
            backend::Message::Pong(v) => Self::Pong(v),
            backend::Message::Close(v) => Self::Close(v.map(|v| CloseFrame {
                code: v.code,
                reason: Utf8Bytes(v.reason),
            })),
        }
    }
    fn into_backend(self) -> backend::Message {
        match self {
            Self::Text(v) => backend::Message::Text(v.0),
            Self::Binary(v) => backend::Message::Binary(v),
            Self::Ping(v) => backend::Message::Ping(v),
            Self::Pong(v) => backend::Message::Pong(v),
            Self::Close(v) => backend::Message::Close(v.map(|v| backend::CloseFrame {
                code: v.code,
                reason: v.reason.0,
            })),
        }
    }
}

/// A bidirectional connection, supporting `StreamExt::split` and `SinkExt`.
#[derive(Debug)]
pub struct WebSocket(backend::WebSocket);
impl WebSocket {
    pub async fn recv(&mut self) -> Option<Result<Message, WebSocketError>> {
        self.next().await
    }
    pub async fn send(&mut self, message: Message) -> Result<(), WebSocketError> {
        SinkExt::send(self, message).await
    }
    /// Flush a close frame. Continue receiving to complete the peer handshake.
    pub async fn close(&mut self) -> Result<(), WebSocketError> {
        SinkExt::close(self).await
    }
    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.0.protocol()
    }
}
impl Stream for WebSocket {
    type Item = Result<Message, WebSocketError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().0)
            .poll_next(cx)
            .map(|v| v.map(|v| v.map(Message::from_backend).map_err(WebSocketError::new)))
    }
}
impl FusedStream for WebSocket {
    fn is_terminated(&self) -> bool {
        self.0.is_terminated()
    }
}
impl Sink<Message> for WebSocket {
    type Error = WebSocketError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().0)
            .poll_ready(cx)
            .map_err(WebSocketError::new)
    }
    fn start_send(self: Pin<&mut Self>, value: Message) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().0)
            .start_send(value.into_backend())
            .map_err(WebSocketError::new)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().0)
            .poll_flush(cx)
            .map_err(WebSocketError::new)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().0)
            .poll_close(cx)
            .map_err(WebSocketError::new)
    }
}

type FailedUpgrade = Box<dyn FnOnce(axum::Error) + Send + 'static>;
/// Request-head extractor. Return `on_upgrade`'s response to accept the connection.
/// HTTP/1.1 GET and HTTP/2 CONNECT validation retain the backend's behavior.
#[must_use]
pub struct WebSocketUpgrade(backend::WebSocketUpgrade<FailedUpgrade>);
impl<S: Send + Sync> super::FromRequestParts<S> for WebSocketUpgrade {
    type Rejection = RejectionResponse;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRequestParts;
        backend::WebSocketUpgrade::from_request_parts(parts, state)
            .await
            .map(|v| Self(v.on_failed_upgrade(Box::new(|_| {}) as FailedUpgrade)))
            .map_err(|e| super::extract::rejection(e.status(), e.body_text()))
    }
}
impl WebSocketUpgrade {
    /// Read buffer in bytes (default 128 KiB).
    pub fn read_buffer_size(self, size: usize) -> Self {
        Self(self.0.read_buffer_size(size))
    }
    /// Target buffered writes in bytes (default 128 KiB); zero writes eagerly.
    pub fn write_buffer_size(self, size: usize) -> Self {
        Self(self.0.write_buffer_size(size))
    }
    /// Backpressure bound (default unlimited). Must exceed write buffer size;
    /// invalid buffer configuration panics when the transport is constructed.
    pub fn max_write_buffer_size(self, size: usize) -> Self {
        Self(self.0.max_write_buffer_size(size))
    }
    /// Maximum reassembled message bytes (default 64 MiB).
    pub fn max_message_size(self, size: usize) -> Self {
        Self(self.0.max_message_size(size))
    }
    /// Maximum individual frame bytes (default 16 MiB).
    pub fn max_frame_size(self, size: usize) -> Self {
        Self(self.0.max_frame_size(size))
    }
    /// Accept unmasked client frames (default false). Enable only deliberately.
    pub fn accept_unmasked_frames(self, accept: bool) -> Self {
        Self(self.0.accept_unmasked_frames(accept))
    }
    /// Select the first server-preferred protocol offered by the client.
    /// No match permits an upgrade without a protocol; enforce requirements in the handler.
    pub fn protocols<I>(self, protocols: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'static, str>>,
    {
        Self(self.0.protocols(protocols))
    }
    pub fn requested_protocols(&self) -> impl Iterator<Item = &HeaderValue> {
        self.0.requested_protocols()
    }
    pub fn selected_protocol(&self) -> Option<&HeaderValue> {
        self.0.selected_protocol()
    }
    /// Override negotiation. Caller must ensure the client offered this protocol.
    pub fn set_selected_protocol(&mut self, protocol: HeaderValue) {
        self.0.set_selected_protocol(protocol);
    }
    /// Observe background upgrade failures; extractor rejections are returned separately.
    pub fn on_failed_upgrade(self, callback: impl FnOnce(WebSocketError) + Send + 'static) -> Self {
        Self(
            self.0.on_failed_upgrade(
                Box::new(move |e| callback(WebSocketError::new(e))) as FailedUpgrade
            ),
        )
    }
    #[must_use = "return this response to accept the upgrade"]
    pub fn on_upgrade<C, F>(self, callback: C) -> Response
    where
        C: FnOnce(WebSocket) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.0
            .on_upgrade(move |socket| callback(WebSocket(socket)))
            .map(super::Body)
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
