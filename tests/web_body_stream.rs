#![cfg(feature = "web")]

use futures_util::StreamExt;
use http_body::{Frame, SizeHint};
use simple_server::web::body::{Body, Bytes, HttpBody};
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

struct Frames {
    frames: VecDeque<Result<Frame<Bytes>, std::io::Error>>,
    polls: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
}
impl Drop for Frames {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
impl HttpBody for Frames {
    type Data = Bytes;
    type Error = std::io::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(self.frames.pop_front())
    }
    fn size_hint(&self) -> SizeHint {
        SizeHint::default()
    }
}
fn frames(polls: Arc<AtomicUsize>, dropped: Arc<AtomicBool>) -> Frames {
    let mut trailers = simple_server::web::HeaderMap::new();
    trailers.insert("x-trailer", "ignored".parse().unwrap());
    Frames {
        polls,
        dropped,
        frames: VecDeque::from([
            Ok(Frame::data(Bytes::from_static(b"first"))),
            Ok(Frame::trailers(trailers)),
            Ok(Frame::data(Bytes::from_static(b"second"))),
            Err(std::io::Error::other("stream failure")),
        ]),
    }
}

#[tokio::test]
async fn data_stream_matches_backend_frames_trailers_and_errors() {
    let source = || frames(Arc::default(), Arc::default());
    let actual: Vec<_> = Body::new(source())
        .into_data_stream()
        .map(|item| item.map_err(|error| error.to_string()))
        .collect()
        .await;
    let expected: Vec<_> = simple_server::axum::body::Body::new(source())
        .into_data_stream()
        .map(|item| item.map_err(|error| error.to_string()))
        .collect()
        .await;
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0].as_ref().unwrap(), b"first".as_slice());
    assert_eq!(actual[1].as_ref().unwrap(), b"second".as_slice());
    assert_eq!(actual[2].as_ref().unwrap_err(), "stream failure");
}

#[tokio::test]
async fn data_stream_is_lazy_and_releases_unread_body_on_drop() {
    let polls = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicBool::new(false));
    let mut stream = Body::new(frames(polls.clone(), dropped.clone())).into_data_stream();
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        Bytes::from_static(b"first")
    );
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    drop(stream);
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(polls.load(Ordering::SeqCst), 1);
}
