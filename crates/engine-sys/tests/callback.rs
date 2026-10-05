use simple_server_sys::{Builder, Callback, Operation, frame, sleep, unframe};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

fn invoke(callback: &Callback, body: &[u8]) -> Operation {
    Operation::new(
        &frame(
            format!("{{\"op\":\"callback\",\"callback\":{}}}", callback.id()).as_bytes(),
            body,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn callbacks_can_use_engine_runtime_and_survive_registration_release() {
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let callback = Callback::new(|mut input| async move {
                sleep(Duration::from_secs(3600)).await;
                input.extend_from_slice(&[0, 255]);
                input
            })
            .unwrap();
            let operation = invoke(&callback, b"binary");
            drop(callback);
            let output = operation.await.unwrap();
            assert_eq!(unframe(&output).unwrap().1, b"binary\0\xff");
        });
}

#[test]
fn callback_cancellation_drops_the_host_future_exactly_once() {
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let callback_count = count.clone();
            let callback = Callback::new(move |_| {
                let guard = Guard(callback_count.clone());
                async move {
                    let _guard = guard;
                    std::future::pending().await
                }
            })
            .unwrap();
            let mut operation = Box::pin(invoke(&callback, b""));
            std::future::poll_fn(|cx| {
                assert!(operation.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            drop(operation);
            assert_eq!(count.load(Ordering::SeqCst), 1);
            drop(callback);
            assert_eq!(count.load(Ordering::SeqCst), 1);
        });
}

#[test]
fn synchronous_and_async_handler_panics_stay_inside_the_host() {
    Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let synchronous =
                Callback::new(|_| -> std::future::Ready<Vec<u8>> { panic!("before future") })
                    .unwrap();
            let asynchronous = Callback::new(|_| async { panic!("during poll") }).unwrap();
            for callback in [synchronous, asynchronous] {
                let output = invoke(&callback, b"").await.unwrap();
                assert!(
                    String::from_utf8_lossy(unframe(&output).unwrap().0)
                        .contains("application callback panicked")
                );
            }
        });
}
