#![cfg(all(
    feature = "engine-web",
    feature = "client",
    feature = "runtime-primitives"
))]
use simple_server::{
    client::{blocking, multipart},
    engine_lifecycle::Shutdown,
    engine_web as web, runtime,
};
use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
struct Reader {
    remaining: usize,
    dropped: Arc<AtomicBool>,
}
impl Read for Reader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        assert!(buffer.len() <= 64 * 1024);
        let n = buffer.len().min(self.remaining);
        buffer[..n].fill(42);
        self.remaining -= n;
        Ok(n)
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
#[simple_server::test]
async fn blocking_multipart_streams_bounded_reader_and_preserves_metadata() {
    let app = web::Router::new()
        .unwrap()
        .route(
            "/",
            web::MethodRouter::new()
                .unwrap()
                .on_handler(
                    web::Method::POST,
                    |mut form: web::multipart::Multipart| async move {
                        let mut file = form.next_field().await.unwrap().unwrap();
                        assert_eq!(file.name(), Some("file"));
                        assert_eq!(file.file_name(), Some("audio.mp3"));
                        assert_eq!(file.content_type(), Some("audio/mpeg"));
                        let mut length = 0;
                        while let Some(chunk) = file.chunk().await.unwrap() {
                            assert!(chunk.iter().all(|b| *b == 42));
                            length += chunk.len();
                        }
                        assert_eq!(length, 200_000);
                        drop(file);
                        let field = form.next_field().await.unwrap().unwrap();
                        assert_eq!(field.name(), Some("options"));
                        assert_eq!(field.text().await.unwrap(), "{\"λ\":true}");
                        assert!(form.next_field().await.unwrap().is_none());
                        "uploaded"
                    },
                )
                .unwrap(),
        )
        .unwrap();
    let listener = web::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr());
    let stop = Shutdown::new();
    let task = runtime::spawn(web::serve(listener, app, stop.clone()));
    let dropped = Arc::new(AtomicBool::new(false));
    let reader = Reader {
        remaining: 200_000,
        dropped: dropped.clone(),
    };
    let result = runtime::spawn_blocking(move || {
        blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
            .post(url)
            .multipart(
                multipart::Form::new()
                    .part(
                        "file",
                        multipart::Part::reader_with_length(reader, 200_000)
                            .file_name("audio.mp3")
                            .mime_str("audio/mpeg")
                            .unwrap(),
                    )
                    .text("options", "{\"λ\":true}"),
            )
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .text()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(result, "uploaded");
    assert!(dropped.load(Ordering::SeqCst));
    stop.request();
    task.await.unwrap().unwrap();
}
#[simple_server::test]
async fn cancellation_children_are_one_way_and_blocking_handle_drives_engine() {
    use simple_server::primitives::CancellationToken;
    let parent = CancellationToken::new();
    let child = parent.child_token();
    let sibling = parent.child_token();
    child.cancel();
    assert!(child.is_cancelled());
    assert!(!parent.is_cancelled());
    assert!(!sibling.is_cancelled());
    let wait = runtime::spawn({
        let sibling = sibling.clone();
        async move { sibling.cancelled().await }
    });
    parent.cancel();
    wait.await.unwrap();
    assert!(sibling.is_cancelled());
    let handle = runtime::Handle::current();
    assert_eq!(
        runtime::spawn_blocking(move || handle.block_on(async {
            simple_server::time::sleep(Duration::from_millis(1)).await;
            42
        }))
        .await
        .unwrap(),
        42
    );
}
