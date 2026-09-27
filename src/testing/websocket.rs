use super::{TestError, TestRequest, TestResponse, error};
use crate::web::ws::{CloseFrame, Message};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{self, client::IntoClientRequest},
};

/// Upgrade rejection exposes HTTP status/headers. Its body contains only bytes
/// received with the handshake; use an ordinary HTTP request for full-body assertions.
pub enum WebSocketOutcome {
    Connected(TestWebSocket),
    Rejected(TestResponse),
}
/// Real TCP WebSocket client with owned message types and bounded operations.
pub struct TestWebSocket {
    socket: Box<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>,
    headers: http::HeaderMap,
    timeout: Duration,
}
impl TestRequest<'_> {
    /// TCP fixtures only. Headers, including authentication and subprotocols, are forwarded.
    pub async fn websocket(self) -> Result<WebSocketOutcome, TestError> {
        if self.server.address().is_none() {
            return Err(error("WebSocket testing requires a TCP fixture"));
        }
        let options = self.server.options;
        let request = self.build()?;
        if request.method() != http::Method::GET || request.body().is_some() {
            return Err(error("WebSocket handshake requires GET without a body"));
        }
        let url = request.url().as_str().replacen("http://", "ws://", 1);
        let mut handshake = url.into_client_request()?;
        for (name, value) in request.headers() {
            handshake.headers_mut().append(name, value.clone());
        }
        let mut config = tungstenite::protocol::WebSocketConfig::default();
        config.max_message_size = Some(options.max_response_bytes);
        config.max_frame_size = Some(options.max_response_bytes);
        match tokio::time::timeout(
            options.timeout,
            tokio_tungstenite::connect_async_with_config(handshake, Some(config), false),
        )
        .await?
        {
            Ok((socket, response)) => Ok(WebSocketOutcome::Connected(TestWebSocket {
                socket: Box::new(socket),
                headers: response.headers().clone(),
                timeout: options.timeout,
            })),
            Err(tungstenite::Error::Http(response)) => {
                let (parts, body) = response.into_parts();
                let body = body.unwrap_or_default();
                if body.len() > options.max_response_bytes {
                    return Err(error("upgrade rejection exceeds test byte limit"));
                }
                Ok(WebSocketOutcome::Rejected(TestResponse {
                    status: parts.status,
                    headers: parts.headers,
                    body: body.into(),
                }))
            }
            Err(e) => Err(e.into()),
        }
    }
}
impl TestWebSocket {
    pub fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }
    pub async fn send(&mut self, message: Message) -> Result<(), TestError> {
        let message = match message {
            Message::Text(s) => tungstenite::Message::Text(s.as_str().into()),
            Message::Binary(b) => tungstenite::Message::Binary(b),
            Message::Ping(b) => tungstenite::Message::Ping(b),
            Message::Pong(b) => tungstenite::Message::Pong(b),
            Message::Close(frame) => {
                tungstenite::Message::Close(frame.map(|f| tungstenite::protocol::CloseFrame {
                    code: f.code.into(),
                    reason: f.reason.as_str().into(),
                }))
            }
        };
        tokio::time::timeout(self.timeout, self.socket.send(message)).await??;
        Ok(())
    }
    /// EOF is `None`; ping/pong and close frames remain visible to the test.
    pub async fn receive(&mut self) -> Result<Option<Message>, TestError> {
        loop {
            let Some(message) = tokio::time::timeout(self.timeout, self.socket.next()).await?
            else {
                return Ok(None);
            };
            return Ok(Some(match message? {
                tungstenite::Message::Text(s) => Message::text(s.as_str()),
                tungstenite::Message::Binary(b) => Message::Binary(b),
                tungstenite::Message::Ping(b) => Message::Ping(b),
                tungstenite::Message::Pong(b) => Message::Pong(b),
                tungstenite::Message::Close(frame) => Message::Close(frame.map(|f| CloseFrame {
                    code: f.code.into(),
                    reason: f.reason.as_str().into(),
                })),
                tungstenite::Message::Frame(_) => continue,
            }));
        }
    }
    pub async fn close(&mut self) -> Result<(), TestError> {
        tokio::time::timeout(self.timeout, self.socket.as_mut().close(None)).await??;
        Ok(())
    }
}
