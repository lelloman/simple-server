use simple_server_sys::{
    Builder, Handle, Id, JoinSet, Runtime, advance, sleep, timeout, yield_now,
};
use std::{
    collections::BTreeSet,
    future::{Future, poll_fn},
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
    time::Duration,
};
struct Guard(Arc<AtomicUsize>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn paused() -> Runtime {
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
}

#[test]
fn ids_match_handles_results_panics_and_cancellation() {
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        let ok = tasks.spawn(async { 42 });
        let panic = tasks.spawn(async { std::panic::panic_any(73_u32) });
        let cancel = tasks.spawn(std::future::pending::<i32>());
        cancel.abort();
        let mut seen = BTreeSet::<Id>::new();
        while let Some(result) = tasks.join_next_with_id().await {
            match result {
                Ok((id, value)) => {
                    assert_eq!(id, ok.id());
                    assert_eq!(value, 42);
                    assert!(seen.insert(id));
                }
                Err(error) => {
                    let id = error.id();
                    assert!(seen.insert(id));
                    if id == panic.id() {
                        assert!(error.is_panic());
                        assert_eq!(*error.into_panic().downcast::<u32>().unwrap(), 73);
                    } else {
                        assert_eq!(id, cancel.id());
                        assert!(error.is_cancelled());
                        assert!(error.try_into_panic().is_err());
                    }
                }
            }
        }
        assert_eq!(seen.len(), 3);
        assert!(ok.is_finished() && panic.is_finished() && cancel.is_finished());
        assert!(tasks.is_empty());
        assert!(tasks.try_join_next().is_none());
    });
}
#[test]
fn dropping_wait_is_cancel_safe_and_ready_tasks_do_not_wait_for_older_tasks() {
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        let slow = tasks.spawn(async {
            sleep(Duration::from_secs(20)).await;
            20
        });
        let fast = tasks.spawn(async {
            sleep(Duration::from_secs(10)).await;
            10
        });
        assert!(
            timeout(Duration::from_secs(1), tasks.join_next())
                .await
                .is_err()
        );
        assert_eq!(tasks.len(), 2);
        assert_eq!(
            tasks.join_next_with_id().await.unwrap().unwrap(),
            (fast.id(), 10)
        );
        assert!(!slow.is_finished());
        assert_eq!(
            tasks.join_next_with_id().await.unwrap().unwrap(),
            (slow.id(), 20)
        );
        assert!(tasks.join_next().await.is_none());
    });
}
#[test]
fn abort_all_retains_outcomes_and_shutdown_waits_for_destruction() {
    let dropped = Arc::new(AtomicUsize::new(0));
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        for _ in 0..100 {
            let guard = Guard(dropped.clone());
            tasks.spawn(async move {
                let _guard = guard;
                std::future::pending::<()>().await
            });
        }
        tasks.abort_all();
        assert_eq!(tasks.len(), 100);
        for _ in 0..100 {
            assert!(tasks.join_next().await.unwrap().unwrap_err().is_cancelled());
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 100);
        for _ in 0..5 {
            let guard = Guard(dropped.clone());
            tasks.spawn(async move {
                let _guard = guard;
                std::future::pending::<()>().await
            });
        }
        tasks.shutdown().await;
        assert_eq!(dropped.load(Ordering::SeqCst), 105);
        assert!(tasks.is_empty());
    });
}
#[test]
fn drop_aborts_but_detach_keeps_tasks_running_and_set_can_be_reused() {
    let dropped = Arc::new(AtomicUsize::new(0));
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        let guard = Guard(dropped.clone());
        let abort = tasks.spawn(async move {
            let _guard = guard;
            std::future::pending::<()>().await
        });
        drop(tasks);
        while !abort.is_finished() {
            yield_now().await;
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        let mut tasks = JoinSet::new();
        let guard = Guard(dropped.clone());
        let detached = tasks.spawn(async move {
            let _guard = guard;
            sleep(Duration::from_secs(10)).await;
        });
        poll_fn(|cx| {
            assert!(tasks.poll_join_next(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        tasks.detach_all();
        assert!(tasks.is_empty());
        let replacement = tasks.spawn(async {});
        assert_eq!(
            tasks.join_next_with_id().await.unwrap().unwrap().0,
            replacement.id()
        );
        assert!(!detached.is_finished());
        advance(Duration::from_secs(11)).await;
        while !detached.is_finished() {
            yield_now().await;
        }
        assert!(tasks.try_join_next().is_none());
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    });
}
#[test]
fn nonblocking_join_handles_completion_before_and_after_initial_registration() {
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        let first = tasks.spawn(async { 1 });
        let second = tasks.spawn(async {
            sleep(Duration::from_secs(1)).await;
            2
        });
        yield_now().await;
        assert_eq!(tasks.len(), 2);
        assert_eq!(
            tasks.try_join_next_with_id().unwrap().unwrap(),
            (first.id(), 1)
        );
        assert!(tasks.try_join_next().is_none());
        assert_eq!(tasks.len(), 1);
        advance(Duration::from_secs(2)).await;
        while !second.is_finished() {
            yield_now().await;
        }
        assert_eq!(tasks.try_join_next().unwrap().unwrap(), 2);
    });
}
#[test]
fn completion_queue_survives_concurrent_wakes_and_waiter_replacement() {
    Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap()
        .block_on(async {
            let mut tasks = JoinSet::new();
            let mut expected = BTreeSet::new();
            for value in 0..2000 {
                expected.insert(
                    tasks
                        .spawn(async move {
                            yield_now().await;
                            value
                        })
                        .id(),
                );
            }
            {
                let mut waiter = pin!(tasks.join_next_with_id());
                let result = poll_fn(|cx| Poll::Ready(waiter.as_mut().poll(cx))).await;
                if let Poll::Ready(Some(result)) = result {
                    assert!(expected.remove(&result.unwrap().0));
                }
            }
            while let Some(result) = timeout(Duration::from_secs(10), tasks.join_next_with_id())
                .await
                .unwrap()
            {
                assert!(expected.remove(&result.unwrap().0));
            }
            assert!(expected.is_empty());
        });
}
#[test]
fn running_blocking_task_is_not_pretended_cancelled() {
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        let started = Arc::new(AtomicUsize::new(0));
        let signal = started.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let abort = tasks.spawn_blocking(move || {
            signal.store(1, Ordering::SeqCst);
            receive.recv().unwrap()
        });
        while started.load(Ordering::SeqCst) == 0 {
            yield_now().await;
        }
        abort.abort();
        assert!(!abort.is_finished());
        assert!(tasks.try_join_next().is_none());
        send.send(17).unwrap();
        assert_eq!(
            tasks.join_next_with_id().await.unwrap().unwrap(),
            (abort.id(), 17)
        );
    });
}
#[test]
fn set_observes_tasks_from_distinct_runtimes_and_owner_shutdown() {
    let first = Builder::new_current_thread().build().unwrap();
    let second = Builder::new_current_thread().build().unwrap();
    let mut tasks = JoinSet::new();
    let successful = tasks.spawn_on(async { 19 }, &first.handle());
    let cancelled = tasks.spawn_on(std::future::pending::<i32>(), &second.handle());
    let blocking = tasks.spawn_blocking_on(|| 23, &first.handle());
    drop(second);
    first.block_on(async {
        let mut seen = BTreeSet::new();
        while let Some(result) = tasks.join_next_with_id().await {
            match result {
                Ok((id, value)) => {
                    assert!(seen.insert(id));
                    assert!(id == successful.id() || id == blocking.id());
                    assert_eq!(value, if id == successful.id() { 19 } else { 23 });
                }
                Err(error) => {
                    assert!(error.is_cancelled());
                    assert_eq!(error.id(), cancelled.id());
                    assert!(seen.insert(error.id()));
                }
            }
        }
        assert_eq!(seen.len(), 3);
    });
}
#[test]
fn panicking_waiter_does_not_poison_collection_or_cross_ffi() {
    struct PanickingWake;
    impl std::task::Wake for PanickingWake {
        fn wake(self: Arc<Self>) {
            panic!("application waiter");
        }
    }
    paused().block_on(async {
        let mut tasks = JoinSet::new();
        let task = tasks.spawn(async {
            sleep(Duration::from_secs(1)).await;
            37
        });
        let waker = std::task::Waker::from(Arc::new(PanickingWake));
        assert!(
            tasks
                .poll_join_next(&mut std::task::Context::from_waker(&waker))
                .is_pending()
        );
        yield_now().await;
        advance(Duration::from_secs(2)).await;
        while !task.is_finished() {
            yield_now().await;
        }
        assert_eq!(tasks.join_next().await.unwrap().unwrap(), 37);
        assert!(tasks.is_empty());
    });
}
#[test]
fn runtime_handle_spawns_from_foreign_threads_and_expires_with_owner() {
    assert!(Handle::try_current().is_err());
    let runtime = Builder::new_current_thread().build().unwrap();
    let handle = runtime.handle();
    let caller = handle.clone();
    let (task, blocking) = std::thread::spawn(move || {
        assert!(Handle::try_current().is_err());
        (
            caller.spawn(async {
                assert!(Handle::try_current().is_ok());
                yield_now().await;
                31
            }),
            caller.spawn_blocking(|| {
                assert!(Handle::try_current().is_ok());
                41
            }),
        )
    })
    .join()
    .unwrap();
    runtime.block_on(async {
        assert!(Handle::try_current().is_ok());
        assert_eq!(task.await.unwrap(), 31);
        assert_eq!(blocking.await.unwrap(), 41);
    });
    assert!(Handle::try_current().is_err());
    drop(runtime);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle.spawn(async {});
        }))
        .is_err()
    );
}
#[test]
fn task_identity_survives_runtime_shutdown() {
    let runtime = Builder::new_current_thread().build().unwrap();
    let task = runtime.handle().spawn(std::future::pending::<()>());
    let id = task.id();
    let abort = task.abort_handle();
    assert_eq!(abort.id(), id);
    drop(runtime);
    Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let error = task.await.unwrap_err();
            assert!(error.is_cancelled());
            assert_eq!(error.id(), id);
            assert!(abort.is_finished());
        });
}
