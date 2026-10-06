use super::wire;
use bytes::Bytes;
use futures_util::{Stream, TryStream, TryStreamExt};
use http_body::{Body as HttpBody, Frame, SizeHint};
use http_body_util::{BodyExt, Full, Limited, StreamBody, combinators::UnsyncBoxBody};
use serde_json::{Value, json};
use simple_server_sys::{Callback, Operation, Reply, Resource};
use std::{
    fmt,
    future::{Future, poll_fn},
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
#[derive(Debug)]
pub struct BodyError(BoxError);
impl fmt::Display for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for BodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}
/// A lazy, owned HTTP body. No Axum or Tokio body representation is exposed.
pub struct Body(UnsyncBoxBody<Bytes, BoxError>);
impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body")
            .field("size_hint", &self.size_hint())
            .finish_non_exhaustive()
    }
}
impl Body {
    pub fn new<B>(body: B) -> Self
    where
        B: HttpBody<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Self(body.map_err(Into::into).boxed_unsync())
    }
    pub fn empty() -> Self {
        Self::from(Bytes::new())
    }
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: TryStream + Send + 'static,
        S::Ok: Into<Bytes>,
        S::Error: Into<BoxError>,
    {
        Self::new(StreamBody::new(
            stream.map_ok(|data| Frame::data(data.into())),
        ))
    }
    pub async fn collect(self, limit: usize) -> Result<Bytes, BodyError> {
        Limited::new(self, limit)
            .collect()
            .await
            .map(|body| body.to_bytes())
            .map_err(BodyError)
    }
    /// Lazily yield data, discarding trailer frames. Drop releases the source.
    pub fn into_data_stream(self) -> impl Stream<Item = Result<Bytes, BodyError>> + Send {
        BodyExt::into_data_stream(self)
    }
    pub(super) fn incoming(resource: Resource, header: &Value) -> io::Result<Self> {
        let mut hint = SizeHint::new();
        let lower = header["lower"].as_u64().unwrap_or(0);
        let upper = header["upper"].as_u64();
        if upper.is_some_and(|upper| upper < lower) {
            return Err(io::Error::other("invalid body size hint"));
        }
        hint.set_lower(lower);
        if let Some(upper) = upper {
            hint.set_upper(upper);
        }
        Ok(Self::new(Incoming {
            resource,
            pending: None,
            hint,
            ended: false,
        }))
    }
}
impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}
macro_rules! from_body {
    ($($ty:ty),*) => {$(impl From<$ty> for Body { fn from(value: $ty) -> Self { Self::new(Full::new(Bytes::from(value))) } })*};
}
from_body!(Bytes, Vec<u8>, String, &'static str, &'static [u8]);
impl HttpBody for Body {
    type Data = Bytes;
    type Error = BodyError;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, BodyError>>> {
        Pin::new(&mut self.0)
            .poll_frame(cx)
            .map(|frame| frame.map(|result| result.map_err(BodyError)))
    }
    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }
}
struct Incoming {
    resource: Resource,
    pending: Option<Operation>,
    hint: SizeHint,
    ended: bool,
}
impl Incoming {
    fn decode_frame(&mut self, bytes: &[u8]) -> io::Result<Option<Frame<Bytes>>> {
        let (header, payload) = wire::decode(bytes)?;
        match header["kind"].as_str() {
            Some("data") => {
                let mut hint = SizeHint::new();
                hint.set_lower(self.hint.lower().saturating_sub(payload.len() as u64));
                if let Some(upper) = self.hint.upper() {
                    hint.set_upper(upper.saturating_sub(payload.len() as u64));
                }
                self.hint = hint;
                Ok(Some(Frame::data(Bytes::copy_from_slice(payload))))
            }
            Some("trailers") => {
                self.ended = true;
                Ok(Some(Frame::trailers(wire::read_headers(
                    &header["headers"],
                )?)))
            }
            Some("end") => {
                self.ended = true;
                Ok(None)
            }
            Some("error") => Err(io::Error::other(
                header["message"].as_str().unwrap_or("engine body failed"),
            )),
            _ => Err(io::Error::other("invalid engine body frame")),
        }
    }
}
impl HttpBody for Incoming {
    type Data = Bytes;
    type Error = io::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        if self.ended {
            return Poll::Ready(None);
        }
        if self.pending.is_none() {
            let operation = wire::encode(
                json!({"op":"server_body_frame","body":self.resource.id()}),
                &[],
            )
            .and_then(|bytes| Operation::new(&bytes));
            match operation {
                Ok(operation) => self.pending = Some(operation),
                Err(error) => {
                    self.ended = true;
                    return Poll::Ready(Some(Err(error)));
                }
            }
        }
        let bytes = match Pin::new(self.pending.as_mut().unwrap()).poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(bytes) => bytes,
        };
        self.pending = None;
        let result = bytes.and_then(|bytes| self.decode_frame(&bytes));
        if result.is_err() {
            self.ended = true;
        }
        if self.ended {
            self.hint = SizeHint::with_exact(0);
        }
        Poll::Ready(result.transpose())
    }
    fn is_end_stream(&self) -> bool {
        self.ended
    }
    fn size_hint(&self) -> SizeHint {
        self.hint
    }
}

pub(super) fn response_context(response: super::Response, context: u64) -> io::Result<Reply> {
    let (parts, body) = response.into_parts();
    super::continuation::response_extensions(context, parts.extensions);
    let hint = body.size_hint();
    let callback = export(body)?;
    let bytes = wire::encode(
        json!({"status":parts.status.as_u16(),"headers":wire::headers(&parts.headers),"body":callback.id(),"lower":hint.lower(),"upper":hint.upper()}),
        &[],
    )?;
    Ok(Reply::new(bytes).keep_alive(callback))
}

pub(super) fn export(body: Body) -> io::Result<Callback> {
    let host = super::host_runtime::HostRuntime::capture();
    let body = Arc::new(host.own(Mutex::new(body)));
    Callback::new(move |_| {
        let body = body.clone();
        host.scope(async move {
            let frame = poll_fn(|cx| Pin::new(&mut *body.lock().unwrap()).poll_frame(cx)).await;
            let (header, payload) = match frame {
                None => (json!({"kind":"end"}), Bytes::new()),
                Some(Err(error)) => (
                    json!({"kind":"error","message":error.to_string()}),
                    Bytes::new(),
                ),
                Some(Ok(frame)) => match frame.into_data() {
                    Ok(data) => (json!({"kind":"data"}), data),
                    Err(frame) => match frame.into_trailers() {
                        Ok(headers) => (
                            json!({"kind":"trailers","headers":wire::headers(&headers)}),
                            Bytes::new(),
                        ),
                        Err(_) => (
                            json!({"kind":"error","message":"unsupported body frame"}),
                            Bytes::new(),
                        ),
                    },
                },
            };
            wire::encode(header, &payload).expect("body frame metadata is serializable")
        })
    })
}
