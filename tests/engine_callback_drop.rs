#![cfg(all(feature = "engine-web", feature = "client"))]
use simple_server::engine_web as e;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
#[simple_server::test]
async fn content_length_completion_drops_producer_in_its_engine_runtime() {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };
    struct Producer {
        sent: bool,
        released: Arc<AtomicUsize>,
    }
    impl e::HttpBody for Producer {
        type Data = e::Bytes;
        type Error = std::convert::Infallible;
        fn poll_frame(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<e::Frame<e::Bytes>, Self::Error>>> {
            if self.sent {
                return Poll::Pending;
            }
            self.sent = true;
            Poll::Ready(Some(Ok(e::Frame::data(e::Bytes::from_static(b"body")))))
        }
    }
    impl Drop for Producer {
        fn drop(&mut self) {
            let released = self.released.clone();
            // Mirrors ScT's read-pin cleanup: no EOF poll is required and cleanup
            // starts from Drop on the engine runtime.
            simple_server::runtime::Handle::try_current()
                .expect("producer drop context")
                .spawn(async move {
                    simple_server::time::sleep(Duration::from_millis(1)).await;
                    released.fetch_add(1, Ordering::SeqCst);
                });
        }
    }

    let released = Arc::new(AtomicUsize::new(0));
    let keep = released.clone();
    let router = e::Router::new()
        .unwrap()
        .route(
            "/",
            e::MethodRouter::new()
                .unwrap()
                .on(e::Method::GET, move |_| {
                    let released = keep.clone();
                    async move {
                        e::http::Response::builder()
                            .header("content-length", "4")
                            .body(e::Body::new(Producer {
                                sent: false,
                                released,
                            }))
                            .unwrap()
                    }
                })
                .unwrap(),
        )
        .unwrap();
    let listener = e::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr());
    let stop = e::Shutdown::new();
    let server = simple_server::runtime::spawn(e::serve(listener, router, stop.clone()));
    let client = simple_server::client::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    assert_eq!(
        client.get(url).send().await.unwrap().text().await.unwrap(),
        "body"
    );
    simple_server::time::timeout(Duration::from_secs(3), async {
        while released.load(Ordering::SeqCst) == 0 {
            simple_server::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    stop.request();
    simple_server::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
