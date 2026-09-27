//! Owned server-sent event responses, builders and keepalive configuration.
//!
//! Enable the optional `sse` feature (which also enables `web`). Streams are
//! polled lazily; dropping the response releases its stream. Applications own
//! subscriptions, authorization, event IDs, replay and reconnection policy.
//!
//! ```
//! use std::convert::Infallible;
//! use futures_util::stream;
//! use simple_server::web::sse::{Event, KeepAlive, Sse};
//!
//! async fn events() -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
//!     Sse::new(stream::iter([Ok(Event::default().event("update").id("1").data("ready"))]))
//!         .keep_alive(KeepAlive::default())
//! }
//! ```

use super::{Body, IntoResponse, Response, body::BoxError};
use futures_util::{Stream, StreamExt};
use std::{fmt, time::Duration};

/// An owned SSE event. Fields are encoded in builder call order.
///
/// Data is split into protocol fields at carriage returns and newlines. Event
/// names, IDs and comments must not contain those characters; IDs also reject
/// NUL. Single-value fields cannot be set twice. These invalid builder calls
/// panic, preserving the existing protocol contract. Comments can be repeated.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct Event(axum::response::sse::Event);

impl Event {
    /// Empty comment, encoded as `:\n\n`, used by default keepalives.
    pub const DEFAULT_KEEP_ALIVE: Self = Self(axum::response::sse::Event::DEFAULT_KEEP_ALIVE);

    /// Set text data, splitting multiline content into data fields.
    /// Panics if nonempty data has already been written.
    pub fn data(self, data: impl AsRef<str>) -> Self {
        Self(self.0.data(data))
    }

    /// Set compact JSON data; serialization failures use an owned error.
    /// Panics if nonempty data has already been written.
    pub fn json_data<T: serde::Serialize>(self, data: T) -> Result<Self, EventError> {
        self.0.json_data(data).map(Self).map_err(EventError)
    }

    /// Add a comment. Panics if it contains CR or LF.
    pub fn comment(self, comment: impl AsRef<str>) -> Self {
        Self(self.0.comment(comment))
    }

    /// Set the event name. Panics on CR/LF or a repeated name field.
    pub fn event(self, event: impl AsRef<str>) -> Self {
        Self(self.0.event(event))
    }

    /// Set the reconnect delay hint in whole milliseconds (sub-ms truncated).
    /// Clients own their reconnect policy. Panics on a repeated retry field.
    pub fn retry(self, duration: Duration) -> Self {
        Self(self.0.retry(duration))
    }

    /// Set an event ID, including an empty ID to reset a client's cursor.
    /// Panics on CR/LF/NUL or a repeated ID field.
    pub fn id(self, id: impl AsRef<str>) -> Self {
        Self(self.0.id(id))
    }

    /// Write formatted data incrementally, with multiline escaping preserved.
    pub fn into_data_writer(self) -> EventDataWriter {
        EventDataWriter(self.0.into_data_writer())
    }
}

/// Serialization failure while constructing an event's JSON data.
#[derive(Debug)]
pub struct EventError(axum::Error);
impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl std::error::Error for EventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::error::Error::source(&self.0)
    }
}

/// A formatting writer for one event's data, without backend types.
///
/// The first nonempty write panics if data was already present. Empty writes
/// do nothing. Call `into_event` to finish before yielding the event.
#[derive(Debug)]
#[must_use]
pub struct EventDataWriter(axum::response::sse::EventDataWriter);
impl EventDataWriter {
    pub fn into_event(self) -> Event {
        Event(self.0.into_event())
    }
}
impl fmt::Write for EventDataWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        fmt::Write::write_str(&mut self.0, text)
    }
}

/// Opt-in keepalive configuration. Default: empty comment every 15 idle seconds.
///
/// Real events reset the idle timer; ready events take priority over keepalives.
/// End-of-stream and errors are not replaced by heartbeats. Calling `new` does
/// not start a timer; enabling keepalive on a response requires a Tokio runtime
/// with time enabled when the response is converted into its HTTP body.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct KeepAlive(axum::response::sse::KeepAlive);
impl KeepAlive {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn interval(self, duration: Duration) -> Self {
        Self(self.0.interval(duration))
    }
    /// Customize the comment. Panics if the text contains CR or LF.
    pub fn text(self, text: impl AsRef<str>) -> Self {
        Self(self.0.text(text))
    }
    /// Use a complete event as the heartbeat, including data or named events.
    pub fn event(self, event: Event) -> Self {
        Self(self.0.event(event.0))
    }
}

/// An SSE response with an application-owned fallible event stream.
///
/// Emits HTTP 200, `Content-Type: text/event-stream` and `Cache-Control: no-cache`.
/// No keepalive is enabled unless configured. Each event becomes one body frame;
/// events are not pre-collected, buffered or spawned into background tasks.
/// Stream errors propagate as shared body errors; after headers are sent they
/// cannot become another HTTP status. They are not converted to SSE error events.
/// A dropped response/body drops the stream; producer tasks remain caller-owned.
#[derive(Clone)]
#[must_use]
pub struct Sse<S> {
    stream: S,
    keep_alive: Option<KeepAlive>,
}
impl<S> Sse<S> {
    pub fn new<E>(stream: S) -> Self
    where
        S: Stream<Item = Result<Event, E>> + Send + 'static,
        E: Into<BoxError>,
    {
        Self {
            stream,
            keep_alive: None,
        }
    }
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }
}
impl<S> fmt::Debug for Sse<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sse")
            .field("stream", &std::any::type_name::<S>())
            .field("keep_alive", &self.keep_alive)
            .finish()
    }
}
impl<S, E> IntoResponse for Sse<S>
where
    S: Stream<Item = Result<Event, E>> + Send + 'static,
    E: Into<BoxError>,
{
    fn into_response(self) -> Response {
        let response =
            axum::response::sse::Sse::new(self.stream.map(|event| event.map(|event| event.0)));
        let response = match self.keep_alive {
            Some(keep_alive) => {
                axum::response::IntoResponse::into_response(response.keep_alive(keep_alive.0))
            }
            None => axum::response::IntoResponse::into_response(response),
        };
        response.map(Body)
    }
}
