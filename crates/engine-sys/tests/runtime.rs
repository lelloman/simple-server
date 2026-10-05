use simple_server_sys::{Builder, Runtime, advance, sleep, spawn, yield_now};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[test]
fn root_can_borrow_non_send_state() {
    let runtime = Builder::new_current_thread().build().unwrap();
    let value = std::rc::Rc::new(std::cell::Cell::new(0));
    runtime.block_on(async {
        yield_now().await;
        value.set(42);
    });
    assert_eq!(value.get(), 42);
}

#[test]
fn spawned_tasks_keep_results_and_panics_on_the_host() {
    Runtime::new().unwrap().block_on(async {
        assert_eq!(
            spawn(async {
                sleep(Duration::from_millis(1)).await;
                17
            })
            .await
            .unwrap(),
            17
        );
        let error = spawn(async { std::panic::panic_any(123_u32) })
            .await
            .unwrap_err();
        assert!(error.is_panic());
        assert_eq!(*error.into_panic().downcast::<u32>().unwrap(), 123);
    });
}

#[test]
fn abort_releases_future_once_before_reporting_completion() {
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    Runtime::new().unwrap().block_on(async {
        let guard = Guard(count.clone());
        let task = spawn(async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        });
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(count.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn paused_clock_belongs_to_engine() {
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let task = spawn(async {
                sleep(Duration::from_secs(3600)).await;
                9
            });
            yield_now().await;
            assert!(!task.is_finished());
            advance(Duration::from_secs(3600)).await;
            assert_eq!(task.await.unwrap(), 9);
        });
}

#[test]
fn root_panic_resumes_on_host() {
    let runtime = Builder::new_current_thread().build().unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(async { panic!("root") })
    }));
    assert!(panic.is_err());
    assert_eq!(runtime.block_on(async { 7 }), 7);
}

#[test]
fn blocking_work_runs_off_the_current_thread_and_keeps_panic_payloads() {
    Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let caller = std::thread::current().id();
            let worker = simple_server_sys::spawn_blocking(|| std::thread::current().id())
                .await
                .unwrap();
            assert_ne!(caller, worker);
            let error = simple_server_sys::spawn_blocking(|| std::panic::panic_any(456_u32))
                .await
                .unwrap_err();
            assert_eq!(*error.into_panic().downcast::<u32>().unwrap(), 456);
        });
}

#[test]
fn timeout_uses_engine_clock_and_cancels_the_inner_future() {
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let guard = Guard(count.clone());
            let result = simple_server_sys::timeout(Duration::from_secs(3600), async move {
                let _guard = guard;
                std::future::pending::<()>().await
            })
            .await;
            assert_eq!(result, Err(simple_server_sys::Elapsed));
            assert_eq!(count.load(Ordering::SeqCst), 1);
            assert_eq!(
                simple_server_sys::timeout(Duration::ZERO, async { 11 }).await,
                Ok(11)
            );
        });
}

#[test]
fn application_waker_panic_does_not_unwind_through_the_engine() {
    struct PanickingWake(Arc<AtomicUsize>);
    impl std::task::Wake for PanickingWake {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
            panic!("application waker");
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let waker = std::task::Waker::from(Arc::new(PanickingWake(calls.clone())));
            let mut operation = Box::pin(
                simple_server_sys::Operation::new(b"{\"op\":\"sleep\",\"secs\":1,\"nanos\":0}")
                    .unwrap(),
            );
            assert!(
                operation
                    .as_mut()
                    .poll(&mut std::task::Context::from_waker(&waker))
                    .is_pending()
            );
            advance(Duration::from_secs(2)).await;
            operation.await.unwrap();
            assert!(calls.load(Ordering::SeqCst) > 0);
        });
}
