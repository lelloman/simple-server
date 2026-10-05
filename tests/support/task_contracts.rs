use super::{clock, execution, runtime_test, tasks::*};
use clock::Instant;
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
fn set() -> TaskSet<&'static str> {
    TaskSet::new(NonZeroUsize::new(8).unwrap())
}

#[runtime_test]
async fn reserved_upgrade_blocks_drain_until_callback_or_failed_handshake_drops_it() {
    let tracker = WorkTracker::new();
    let guard = tracker.try_acquire("pending-upgrade").unwrap();
    tracker.close();
    assert_eq!(
        tracker.try_acquire("late").unwrap_err(),
        AdmissionError::Closed
    );
    assert!(
        clock::timeout(Duration::from_millis(5), tracker.wait())
            .await
            .is_err()
    );
    drop(guard);
    tracker.wait().await;
}
#[runtime_test]
async fn open_empty_tracker_does_not_claim_drain() {
    let tracker = WorkTracker::new();
    assert!(
        clock::timeout(Duration::from_millis(5), tracker.wait())
            .await
            .is_err()
    );
    tracker.close();
    tracker.wait().await;
}
#[test]
fn close_races_with_external_admission() {
    for _ in 0..100 {
        let tracker = WorkTracker::new();
        let other = tracker.clone();
        let thread = std::thread::spawn(move || other.try_acquire("racing"));
        tracker.close();
        let guard = thread.join().unwrap();
        assert_eq!(
            tracker.try_acquire("after-close").unwrap_err(),
            AdmissionError::Closed
        );
        if guard.is_ok() {
            assert_eq!(tracker.unfinished().len(), 1);
        }
        drop(guard);
        assert!(tracker.unfinished().is_empty());
    }
}
#[runtime_test]
async fn outcomes_preserve_errors_and_factory_panics_without_stopping_siblings() {
    let mut tasks = set();
    tasks
        .spawn("ok", Default::default(), |_| async { Ok(()) })
        .unwrap();
    tasks
        .spawn("error", Default::default(), |_| async { Err("source") })
        .unwrap();
    tasks
        .spawn("panic", Default::default(), |_| {
            panic!("factory");
            #[allow(unreachable_code)]
            async {
                Ok(())
            }
        })
        .unwrap();
    let report = tasks
        .drain_until(Instant::now() + Duration::from_secs(1))
        .await;
    assert!(report.is_drained());
    assert_eq!(report.completed.len(), 3);
    assert!(
        report
            .completed
            .iter()
            .any(|c| matches!(c.exit, TaskExit::Finished(Err("source"))))
    );
    assert!(
        report
            .completed
            .iter()
            .any(|c| matches!(&c.exit, TaskExit::Panicked(s) if s == "factory"))
    );
}
#[runtime_test]
async fn unconsumed_results_bound_admission_without_executing_rejected_factory() {
    let mut tasks = TaskSet::<()>::new(NonZeroUsize::new(1).unwrap());
    tasks
        .spawn("first", Default::default(), |_| async { Ok(()) })
        .unwrap();
    execution::yield_now().await;
    assert_eq!(
        tasks.spawn("second", Default::default(), |_| {
            panic!("must not execute");
            #[allow(unreachable_code)]
            async {
                Ok(())
            }
        }),
        Err(AdmissionError::Full)
    );
    tasks.join_next().await.unwrap();
    tasks
        .spawn("after-consumption", Default::default(), |_| async {
            Ok(())
        })
        .unwrap();
    tasks.join_next().await.unwrap();
}
#[runtime_test]
async fn timeout_retains_ownership_and_finish_jobs_ignore_shutdown() {
    let mut tasks = set();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tasks
        .spawn("cancel", Default::default(), |ctx| async move {
            ctx.cancelled().await;
            Ok(())
        })
        .unwrap();
    tasks
        .spawn(
            "finalize-claim",
            ShutdownBehavior::FinishOnShutdown,
            |ctx| async move {
                rx.await.unwrap();
                assert!(!ctx.is_cancelled());
                Ok(())
            },
        )
        .unwrap();
    tasks.request_shutdown();
    let first = tasks
        .drain_until(Instant::now() + Duration::from_millis(10))
        .await;
    assert!(!first.is_drained());
    assert_eq!(first.unfinished[0].name, "finalize-claim");
    assert_eq!(
        first.completed[0].cancellation,
        Some(CancellationReason::Shutdown)
    );
    tx.send(()).unwrap();
    assert!(
        tasks
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .is_drained()
    );
}
#[runtime_test]
async fn cancelling_drain_preserves_already_collected_outcomes() {
    let mut tasks = set();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tasks
        .spawn("done", Default::default(), |_| async { Err("retain") })
        .unwrap();
    tasks
        .spawn("held", Default::default(), |_| async {
            rx.await.unwrap();
            Ok(())
        })
        .unwrap();
    assert!(
        clock::timeout(
            Duration::from_millis(10),
            tasks.drain_until(Instant::now() + Duration::from_secs(10))
        )
        .await
        .is_err()
    );
    tx.send(()).unwrap();
    let report = tasks
        .drain_until(Instant::now() + Duration::from_secs(1))
        .await;
    assert_eq!(report.completed.len(), 2);
    assert!(
        report
            .completed
            .iter()
            .any(|c| matches!(c.exit, TaskExit::Finished(Err("retain"))))
    );
}
#[runtime_test]
async fn blocking_work_retains_ownership_and_cannot_be_aborted() {
    let mut tasks = set();
    let (tx, rx) = std::sync::mpsc::channel();
    let (started, ready) = tokio::sync::oneshot::channel();
    let id = tasks
        .spawn_blocking("blocking", Default::default(), move |ctx| {
            started.send(()).unwrap();
            rx.recv().unwrap();
            assert!(ctx.is_cancelled());
            Ok(())
        })
        .unwrap();
    ready.await.unwrap();
    assert_eq!(tasks.abort_async(id), Err(AbortError::BlockingTask));
    tasks.request_shutdown();
    assert!(
        !tasks
            .drain_until(Instant::now() + Duration::from_millis(5))
            .await
            .is_drained()
    );
    tx.send(()).unwrap();
    assert!(
        tasks
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .is_drained()
    );
}
#[runtime_test]
async fn explicit_abort_is_distinct_from_cancellation_request() {
    let mut tasks = set();
    let id = tasks
        .spawn("abort", Default::default(), |_| std::future::pending())
        .unwrap();
    tasks.abort_async(id).unwrap();
    assert!(matches!(
        tasks.join_next().await.unwrap().exit,
        TaskExit::Aborted
    ));
}
#[runtime_test]
async fn dropping_owner_requests_cooperation_without_aborting_execution() {
    let mut tasks = set();
    let finished = Arc::new(AtomicBool::new(false));
    let output = finished.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tasks
        .spawn("detached", Default::default(), |ctx| async move {
            ctx.cancelled().await;
            rx.await.unwrap();
            output.store(true, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
    drop(tasks);
    assert!(!finished.load(Ordering::SeqCst));
    tx.send(()).unwrap();
    clock::timeout(Duration::from_secs(1), async {
        while !finished.load(Ordering::SeqCst) {
            execution::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn admission_without_runtime_does_not_execute_factory() {
    let mut tasks = set();
    assert_eq!(
        tasks.spawn("outside-runtime", Default::default(), |_| {
            panic!("rejected factory ran");
            #[allow(unreachable_code)]
            async {
                Ok(())
            }
        }),
        Err(AdmissionError::NoRuntime)
    );
    assert!(tasks.is_empty());
}

#[runtime_test(start_paused = true)]
async fn timings_record_execution_on_the_selected_clock() {
    let mut tasks = set();
    let start = clock::now();
    let id = tasks
        .spawn("timed", Default::default(), |_| async {
            clock::sleep(Duration::from_secs(3)).await;
            Ok(())
        })
        .unwrap();
    execution::yield_now().await;
    assert_eq!(tasks.timing(id).unwrap().started, Some(start));
    assert!(tasks.timing(id).unwrap().finished.is_none());
    let completion = tasks.join_next().await.unwrap();
    assert_eq!(completion.timing.started, Some(start));
    assert_eq!(
        completion.timing.finished,
        Some(start + Duration::from_secs(3))
    );
    clock::advance(Duration::from_secs(7)).await;
    assert_eq!(
        completion.timing.finished,
        Some(start + Duration::from_secs(3))
    );
}

#[runtime_test]
async fn closing_and_external_completion_wake_all_tracker_waiters() {
    let tracker = WorkTracker::new();
    let guard = tracker.try_acquire("external").unwrap();
    let first = tracker.clone();
    let second = tracker.clone();
    let a = execution::spawn(async move { first.wait().await });
    let b = execution::spawn(async move { second.wait().await });
    execution::yield_now().await;
    tracker.close();
    std::thread::spawn(move || drop(guard)).join().unwrap();
    clock::timeout(Duration::from_secs(1), async {
        a.await.unwrap();
        b.await.unwrap();
        tracker.wait().await;
    })
    .await
    .unwrap();
}
