pub use bytes::Bytes;
pub use http_body::Body as HttpBody;
use http_body::{Frame, SizeHint};
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

/// Owned HTTP body. The backend representation is private.
#[derive(Debug)]
pub struct Body(pub(super) axum::body::Body);

/// A body read failure, including an explicit collection limit being exceeded.
#[derive(Debug)]
pub struct BodyError(axum::Error);

impl fmt::Display for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for BodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn to_bytes(body: Body, limit: usize) -> Result<Bytes, BodyError> {
    body.collect(limit).await
}

impl Body {
    pub fn new<B>(body: B) -> Self
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Self(axum::body::Body::new(body))
    }

    /// Forward a stream lazily, preserving backpressure and cancellation.
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: futures_util::TryStream + Send + 'static,
        S::Ok: Into<Bytes>,
        S::Error: Into<BoxError>,
    {
        Self(axum::body::Body::from_stream(stream))
    }

    /// Consume this body as a lazy data stream. Trailer frames are discarded.
    /// Dropping the stream releases its body and attached resources.
    pub fn into_data_stream(self) -> BodyDataStream {
        BodyDataStream(self)
    }

    pub fn empty() -> Self {
        Self(axum::body::Body::empty())
    }

    /// Collect with an explicit maximum size; no unbounded default is provided.
    pub async fn collect(self, limit: usize) -> Result<Bytes, BodyError> {
        axum::body::to_bytes(self.0, limit).await.map_err(BodyError)
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}

macro_rules! from_body {
    ($($ty:ty),+ $(,)?) => {$(
        impl From<$ty> for Body {
            fn from(value: $ty) -> Self { Self(axum::body::Body::from(value)) }
        }
    )+};
}
from_body!(Bytes, Vec<u8>, String, &'static str, &'static [u8]);

impl http_body::Body for Body {
    type Data = Bytes;
    type Error = BodyError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
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

/// A lazy stream of HTTP data frames with framework-independent errors.
#[derive(Debug)]
pub struct BodyDataStream(Body);

impl futures_util::Stream for BodyDataStream {
    type Item = Result<Bytes, BodyError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.0).poll_frame(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error))),
                Poll::Ready(Some(Ok(frame))) => {
                    if let Ok(data) = frame.into_data() {
                        return Poll::Ready(Some(Ok(data)));
                    }
                }
            }
        }
    }
}
