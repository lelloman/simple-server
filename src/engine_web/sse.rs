//! Server-sent events through the shared engine's streaming body and clock.
//!
//! Available with `engine-web`; the source backend's `sse` feature is not needed.
//! Streams remain application-owned and are polled only as the body is consumed.
//! IDs, authorization, subscriptions and replay policy remain caller-owned.

// Adapted from axum 0.8.9, src/response/sse.rs.
// Uses owned engine bodies and timers; contains no Axum or Tokio runtime types.
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

use super::{Body, Bytes, Frame, HttpBody, IntoResponse, Response, body::BoxError};
use crate::time::{Sleep, sleep};
use bytes::{BufMut, BytesMut};
use futures_util::Stream;
use std::{
    fmt::{self, Write as _},
    future::Future,
    io::Write as _,
    mem,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

/// A JSON serialization failure while building an event.
#[derive(Debug)]
pub struct EventError(serde_json::Error);
impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for EventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// The state of an event's buffer.
///
/// This type allows creating events in a `const` context
/// by using a finalized buffer.
///
/// While the buffer is active, more bytes can be written to it.
/// Once finalized, it's immutable and cheap to clone.
/// The buffer is active during the event building, but eventually
/// becomes finalized to send http body frames as [`Bytes`].
#[derive(Debug, Clone)]
enum Buffer {
    Active(BytesMut),
    Finalized(Bytes),
}

impl Buffer {
    /// Returns a mutable reference to the internal buffer.
    ///
    /// If the buffer was finalized, this method creates
    /// a new active buffer with the previous contents.
    fn as_mut(&mut self) -> &mut BytesMut {
        match self {
            Buffer::Active(bytes_mut) => bytes_mut,
            Buffer::Finalized(bytes) => {
                *self = Buffer::Active(BytesMut::from(mem::take(bytes)));
                match self {
                    Buffer::Active(bytes_mut) => bytes_mut,
                    Buffer::Finalized(_) => unreachable!(),
                }
            }
        }
    }
}

/// Server-sent event
#[derive(Debug, Clone)]
#[must_use]
pub struct Event {
    buffer: Buffer,
    flags: EventFlags,
}

/// Expose [`Event`] as a [`std::fmt::Write`]
/// such that any form of data can be written as data safely.
///
/// This also ensures that newline characters `\r` and `\n`
/// correctly trigger a split with a new `data: ` prefix.
///
/// # Panics
///
/// Panics if any `data` has already been written prior to the first write
/// of this [`EventDataWriter`] instance.
#[derive(Debug)]
#[must_use]
pub struct EventDataWriter {
    event: Event,

    // Indicates if _this_ EventDataWriter has written data,
    // this does not say anything about whether or not `event` contains
    // data or not.
    data_written: bool,
}

impl Event {
    /// Default keep-alive event
    pub const DEFAULT_KEEP_ALIVE: Self = Self::finalized(Bytes::from_static(b":\n\n"));

    const fn finalized(bytes: Bytes) -> Self {
        Self {
            buffer: Buffer::Finalized(bytes),
            flags: EventFlags::from_bits(0),
        }
    }

    /// Use this [`Event`] as a [`EventDataWriter`] to write custom data.
    ///
    /// - [`Self::data`] can be used as a shortcut to write `str` data
    /// - [`Self::json_data`] can be used as a shortcut to write `json` data
    ///
    /// Turn it into an [`Event`] again using [`EventDataWriter::into_event`].
    pub fn into_data_writer(self) -> EventDataWriter {
        EventDataWriter {
            event: self,
            data_written: false,
        }
    }

    /// Set the event's data data field(s) (`data: <content>`)
    ///
    /// Newlines in `data` will automatically be broken across `data: ` fields.
    ///
    /// This corresponds to [`MessageEvent`'s data field].
    ///
    /// Note that events with an empty data field will be ignored by the browser.
    ///
    /// # Panics
    ///
    /// Panics if any `data` has already been written before.
    ///
    /// [`MessageEvent`'s data field]: https://developer.mozilla.org/en-US/docs/Web/API/MessageEvent/data
    pub fn data<T>(self, data: T) -> Self
    where
        T: AsRef<str>,
    {
        let mut writer = self.into_data_writer();
        let _ = writer.write_str(data.as_ref());
        writer.into_event()
    }

    /// Set the event's data field to a value serialized as unformatted JSON (`data: <content>`).
    ///
    /// This corresponds to [`MessageEvent`'s data field].
    ///
    /// # Panics
    ///
    /// Panics if any `data` has already been written before.
    ///
    /// [`MessageEvent`'s data field]: https://developer.mozilla.org/en-US/docs/Web/API/MessageEvent/data
    pub fn json_data<T>(self, data: T) -> Result<Self, EventError>
    where
        T: serde::Serialize,
    {
        struct JsonWriter<'a>(&'a mut EventDataWriter);
        impl std::io::Write for JsonWriter<'_> {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(self.0.write_buf(buf))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut writer = self.into_data_writer();

        let json_writer = JsonWriter(&mut writer);
        serde_json::to_writer(json_writer, &data).map_err(EventError)?;

        Ok(writer.into_event())
    }

    /// Set the event's comment field (`:<comment-text>`).
    ///
    /// This field will be ignored by most SSE clients.
    ///
    /// Unlike other functions, this function can be called multiple times to add many comments.
    ///
    /// # Panics
    ///
    /// Panics if `comment` contains any newlines or carriage returns, as they are not allowed in
    /// comments.
    pub fn comment<T>(mut self, comment: T) -> Event
    where
        T: AsRef<str>,
    {
        self.field("", comment.as_ref());
        self
    }

    /// Set the event's name field (`event:<event-name>`).
    ///
    /// This corresponds to the `type` parameter given when calling `addEventListener` on an
    /// [`EventSource`]. For example, `.event("update")` should correspond to
    /// `.addEventListener("update", ...)`. If no event type is given, browsers will fire a
    /// [`message` event] instead.
    ///
    /// [`EventSource`]: https://developer.mozilla.org/en-US/docs/Web/API/EventSource
    /// [`message` event]: https://developer.mozilla.org/en-US/docs/Web/API/EventSource/message_event
    ///
    /// # Panics
    ///
    /// - Panics if `event` contains any newlines or carriage returns.
    /// - Panics if this function has already been called on this event.
    pub fn event<T>(mut self, event: T) -> Event
    where
        T: AsRef<str>,
    {
        if self.flags.contains(EventFlags::HAS_EVENT) {
            panic!("Called `Event::event` multiple times");
        }
        self.flags.insert(EventFlags::HAS_EVENT);

        self.field("event", event.as_ref());

        self
    }

    /// Set the event's retry timeout field (`retry: <timeout>`).
    ///
    /// This sets how long clients will wait before reconnecting if they are disconnected from the
    /// SSE endpoint. Note that this is just a hint: clients are free to wait for longer if they
    /// wish, such as if they implement exponential backoff.
    ///
    /// # Panics
    ///
    /// Panics if this function has already been called on this event.
    pub fn retry(mut self, duration: Duration) -> Event {
        if self.flags.contains(EventFlags::HAS_RETRY) {
            panic!("Called `Event::retry` multiple times");
        }
        self.flags.insert(EventFlags::HAS_RETRY);

        let buffer = self.buffer.as_mut();
        buffer.extend_from_slice(b"retry: ");

        let secs = duration.as_secs();
        let millis = duration.subsec_millis();

        if secs > 0 {
            // format seconds
            buffer.extend_from_slice(secs.to_string().as_bytes());

            // pad milliseconds
            if millis < 10 {
                buffer.extend_from_slice(b"00");
            } else if millis < 100 {
                buffer.extend_from_slice(b"0");
            }
        }

        // format milliseconds
        buffer.extend_from_slice(millis.to_string().as_bytes());

        buffer.put_u8(b'\n');

        self
    }

    /// Set the event's identifier field (`id:<identifier>`).
    ///
    /// This corresponds to [`MessageEvent`'s `lastEventId` field]. If no ID is in the event itself,
    /// the browser will set that field to the last known message ID, starting with the empty
    /// string.
    ///
    /// [`MessageEvent`'s `lastEventId` field]: https://developer.mozilla.org/en-US/docs/Web/API/MessageEvent/lastEventId
    ///
    /// # Panics
    ///
    /// - Panics if `id` contains any newlines, carriage returns or null characters.
    /// - Panics if this function has already been called on this event.
    pub fn id<T>(mut self, id: T) -> Event
    where
        T: AsRef<str>,
    {
        if self.flags.contains(EventFlags::HAS_ID) {
            panic!("Called `Event::id` multiple times");
        }
        self.flags.insert(EventFlags::HAS_ID);

        let id = id.as_ref().as_bytes();
        assert_eq!(
            id.iter().position(|&byte| byte == b'\0'),
            None,
            "Event ID cannot contain null characters",
        );

        self.field("id", id);
        self
    }

    fn field(&mut self, name: &str, value: impl AsRef<[u8]>) {
        let value = value.as_ref();
        assert_eq!(
            value
                .iter()
                .position(|&byte| byte == b'\r' || byte == b'\n'),
            None,
            "SSE field value cannot contain newlines or carriage returns",
        );

        let buffer = self.buffer.as_mut();
        buffer.extend_from_slice(name.as_bytes());
        buffer.put_u8(b':');
        buffer.put_u8(b' ');
        buffer.extend_from_slice(value);
        buffer.put_u8(b'\n');
    }

    fn finalize(self) -> Bytes {
        match self.buffer {
            Buffer::Finalized(bytes) => bytes,
            Buffer::Active(mut bytes_mut) => {
                bytes_mut.put_u8(b'\n');
                bytes_mut.freeze()
            }
        }
    }
}

impl EventDataWriter {
    /// Consume the [`EventDataWriter`] and return the [`Event`] once again.
    ///
    /// In case any data was written by this instance
    /// it will also write the trailing `\n` character.
    pub fn into_event(self) -> Event {
        let mut event = self.event;
        if self.data_written {
            let _ = event.buffer.as_mut().write_char('\n');
        }
        event
    }
}

impl EventDataWriter {
    // Assumption: underlying writer never returns an error:
    // <https://docs.rs/bytes/latest/src/bytes/buf/writer.rs.html#79-82>
    fn write_buf(&mut self, buf: &[u8]) -> usize {
        if buf.is_empty() {
            return 0;
        }

        let buffer = self.event.buffer.as_mut();

        if !std::mem::replace(&mut self.data_written, true) {
            if self.event.flags.contains(EventFlags::HAS_DATA) {
                panic!("Called `Event::data*` multiple times");
            }

            let _ = buffer.write_str("data: ");
            self.event.flags.insert(EventFlags::HAS_DATA);
        }

        let mut writer = buffer.writer();

        let mut last_split = 0;
        for delimiter in buf
            .iter()
            .enumerate()
            .filter_map(|(i, &byte)| (byte == b'\n' || byte == b'\r').then_some(i))
        {
            let _ = writer.write_all(&buf[last_split..=delimiter]);
            let _ = writer.write_all(b"data: ");
            last_split = delimiter + 1;
        }
        let _ = writer.write_all(&buf[last_split..]);

        buf.len()
    }
}

impl fmt::Write for EventDataWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = self.write_buf(s.as_bytes());
        Ok(())
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            buffer: Buffer::Active(BytesMut::new()),
            flags: EventFlags::from_bits(0),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct EventFlags(u8);

impl EventFlags {
    const HAS_DATA: Self = Self::from_bits(0b0001);
    const HAS_EVENT: Self = Self::from_bits(0b0010);
    const HAS_RETRY: Self = Self::from_bits(0b0100);
    const HAS_ID: Self = Self::from_bits(0b1000);

    const fn bits(&self) -> u8 {
        self.0
    }

    const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    const fn contains(&self, other: Self) -> bool {
        self.bits() & other.bits() == other.bits()
    }

    fn insert(&mut self, other: Self) {
        *self = Self::from_bits(self.bits() | other.bits());
    }
}

/// Configure the interval between keep-alive messages, the content
/// of each message, and the associated stream.
#[derive(Debug, Clone)]
#[must_use]
pub struct KeepAlive {
    event: Event,
    max_interval: Duration,
}

impl KeepAlive {
    /// Create a new `KeepAlive`.
    pub fn new() -> Self {
        Self {
            event: Event::DEFAULT_KEEP_ALIVE,
            max_interval: Duration::from_secs(15),
        }
    }

    /// Customize the interval between keep-alive messages.
    ///
    /// Default is 15 seconds.
    pub fn interval(mut self, time: Duration) -> Self {
        self.max_interval = time;
        self
    }

    /// Customize the text of the keep-alive message.
    ///
    /// Default is an empty comment.
    ///
    /// # Panics
    ///
    /// Panics if `text` contains any newline or carriage returns, as they are not allowed in SSE
    /// comments.
    pub fn text<I>(self, text: I) -> Self
    where
        I: AsRef<str>,
    {
        self.event(Event::default().comment(text))
    }

    /// Customize the event of the keep-alive message.
    ///
    /// Default is an empty comment.
    ///
    pub fn event(mut self, event: Event) -> Self {
        self.event = Event::finalized(event.finalize());
        self
    }
}

impl Default for KeepAlive {
    fn default() -> Self {
        Self::new()
    }
}

/// A lazy SSE response. Emits 200, text/event-stream and no-cache headers.
/// No keepalives are emitted unless configured. Errors remain body errors,
/// and dropping the body drops its stream and any pending engine timer.
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
    /// Enable idle heartbeats. Defaults to an empty comment every 15 seconds.
    /// The timer starts at response conversion, which requires an engine runtime.
    /// Ready events, errors and completion take priority over a due heartbeat.
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
        let keep_alive = self.keep_alive.map(|config| {
            let timer = sleep(config.max_interval);
            (config, timer)
        });
        let body = Body::new(SseBody {
            stream: Box::pin(self.stream),
            keep_alive,
        });
        (
            [
                ("content-type", "text/event-stream"),
                ("cache-control", "no-cache"),
            ],
            body,
        )
            .into_response()
    }
}
struct SseBody<S> {
    stream: Pin<Box<S>>,
    keep_alive: Option<(KeepAlive, Sleep)>,
}
impl<S, E> HttpBody for SseBody<S>
where
    S: Stream<Item = Result<Event, E>>,
    E: Into<BoxError>,
{
    type Data = Bytes;
    type Error = BoxError;
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, BoxError>>> {
        let this = self.get_mut();
        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                if let Some((config, timer)) = &mut this.keep_alive {
                    *timer = sleep(config.max_interval);
                }
                Poll::Ready(Some(Ok(Frame::data(event.finalize()))))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                let Some((config, timer)) = &mut this.keep_alive else {
                    return Poll::Pending;
                };
                if Pin::new(&mut *timer).poll(cx).is_pending() {
                    return Poll::Pending;
                }
                *timer = sleep(config.max_interval);
                Poll::Ready(Some(Ok(Frame::data(config.event.clone().finalize()))))
            }
        }
    }
}
