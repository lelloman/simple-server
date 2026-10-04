#![cfg(feature = "database-blocking")]
use simple_server::database::blocking::*;
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn executor(workers: usize, capacity: usize) -> Executor {
    Executor::new(Config {
        workers: nz(workers),
        priorities: vec![
            Priority {
                weight: nz(2),
                queue_capacity: nz(capacity),
            },
            Priority {
                weight: nz(1),
                queue_capacity: nz(capacity),
            },
        ],
        lane_limits: vec![nz(1), nz(1)],
    })
    .unwrap()
}
fn options(priority: usize, lane: usize) -> Options {
    Options {
        priority,
        lane,
        queue_timeout: None,
        execution_timeout: None,
    }
}
fn blocker(
    executor: &Executor,
    priority: usize,
    lane: usize,
) -> (Ticket<(), ()>, mpsc::Sender<()>) {
    let (started, rx) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let ticket = executor
        .submit(options(priority, lane), move || {
            started.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(())
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(3)).unwrap();
    (ticket, release)
}
fn finish(executor: &Executor) {
    executor.close(CloseMode::Drain);
    assert!(executor.drain_blocking(Duration::from_secs(3)));
}
#[test]
fn weighted_fifo_fairness_under_continuous_higher_priority_backlog() {
    let executor = executor(1, 10);
    let (block, release) = blocker(&executor, 1, 0);
    let (tx, rx) = mpsc::channel();
    let mut tickets = vec![];
    for (priority, values) in [(0, vec![0, 1, 2, 3]), (1, vec![10, 11, 12])] {
        for value in values {
            let tx = tx.clone();
            tickets.push(
                executor
                    .submit(options(priority, 0), move || {
                        tx.send(value).unwrap();
                        Ok::<_, ()>(())
                    })
                    .unwrap(),
            );
        }
    }
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
    for ticket in tickets {
        ticket.wait_blocking().unwrap();
    }
    assert_eq!(
        (0..7)
            .map(|_| rx.recv_timeout(Duration::from_secs(3)).unwrap())
            .collect::<Vec<_>>(),
        vec![0, 1, 10, 2, 3, 11, 12]
    );
    finish(&executor);
}
#[test]
fn saturated_lane_does_not_reserve_an_idle_worker() {
    let executor = executor(2, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let blocked = executor.submit(options(0, 0), || Ok::<_, ()>(1)).unwrap();
    let independent = executor.submit(options(1, 1), || Ok::<_, ()>(2)).unwrap();
    assert_eq!(independent.wait_blocking(), Ok(2));
    assert_eq!(executor.snapshot().active_by_lane[0], 1);
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
    assert_eq!(blocked.wait_blocking(), Ok(1));
    finish(&executor);
}
#[test]
fn bounded_admission_cancellation_and_queue_expiry_never_execute() {
    let executor = executor(1, 1);
    let (block, release) = blocker(&executor, 0, 0);
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let queued = executor
        .submit(options(0, 0), move || {
            c.fetch_add(1, Ordering::SeqCst);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert!(matches!(
        executor.submit(options(0, 0), || Ok::<_, ()>(())),
        Err(SubmitError::QueueFull)
    ));
    drop(queued);
    let c = count.clone();
    let mut opts = options(0, 0);
    opts.queue_timeout = Some(Duration::ZERO);
    let expired = executor
        .submit(opts, move || {
            c.fetch_add(1, Ordering::SeqCst);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(expired.wait_blocking(), Err(RunError::QueueTimeout));
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
    finish(&executor);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
#[test]
fn execution_timeout_keeps_lane_and_worker_until_real_completion() {
    let executor = executor(1, 4);
    let (start, started) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut opts = options(0, 0);
    opts.execution_timeout = Some(Duration::from_millis(20));
    let timed = executor
        .submit(opts, move || {
            start.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok::<_, ()>(())
        })
        .unwrap();
    started.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(timed.wait_blocking(), Err(RunError::ExecutionTimeout));
    let next = executor.submit(options(0, 0), || Ok::<_, ()>(1)).unwrap();
    assert_eq!(executor.snapshot().active, 1);
    assert!(!executor.drain_blocking(Duration::ZERO));
    release.send(()).unwrap();
    assert_eq!(next.wait_blocking(), Ok(1));
    finish(&executor);
}
#[test]
fn late_completed_work_still_reports_execution_timeout() {
    let executor = executor(1, 2);
    let mut opts = options(0, 0);
    opts.execution_timeout = Some(Duration::ZERO);
    let ticket = executor.submit(opts, || Ok::<_, ()>(1)).unwrap();
    finish(&executor);
    assert_eq!(ticket.wait_blocking(), Err(RunError::ExecutionTimeout));
}
#[test]
fn panic_and_native_error_do_not_poison_workers() {
    let executor = executor(1, 4);
    let panic = executor
        .submit(options(0, 0), || -> Result<(), std::io::Error> {
            panic!("intentional fixture")
        })
        .unwrap();
    assert!(matches!(panic.wait_blocking(),Err(RunError::Panicked(s)) if s=="intentional fixture"));
    let error = executor
        .submit(options(0, 0), || {
            Err::<(), _>(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "original",
            ))
        })
        .unwrap();
    assert!(
        matches!(error.wait_blocking(),Err(RunError::Operation(e)) if e.kind()==std::io::ErrorKind::PermissionDenied && e.to_string()=="original")
    );
    assert_eq!(
        executor
            .submit(options(0, 0), || Ok::<_, ()>(7))
            .unwrap()
            .wait_blocking(),
        Ok(7)
    );
    finish(&executor);
}
#[test]
fn close_cancels_queued_work_without_aborting_running_work() {
    let executor = executor(1, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let ticket = executor.submit(options(0, 0), || Ok::<_, ()>(())).unwrap();
    executor.close(CloseMode::CancelQueued);
    assert_eq!(ticket.wait_blocking(), Err(RunError::Cancelled));
    assert!(matches!(
        executor.submit(options(0, 0), || Ok::<_, ()>(())),
        Err(SubmitError::Closed)
    ));
    assert!(!executor.drain_blocking(Duration::ZERO));
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
    assert!(executor.drain_blocking(Duration::from_secs(3)));
}
#[tokio::test]
async fn async_wait_and_drain_share_sync_workers_without_blocking_runtime() {
    let executor = executor(1, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let mut opts = options(0, 0);
    opts.queue_timeout = Some(Duration::from_millis(20));
    let expired = executor.submit(opts, || Ok::<_, ()>(())).unwrap();
    assert_eq!(expired.wait().await, Err(RunError::QueueTimeout));
    let ticket = executor.submit(options(0, 0), || Ok::<_, ()>(42)).unwrap();
    executor.close(CloseMode::Drain);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), executor.drain())
            .await
            .is_err()
    );
    release.send(()).unwrap();
    block.wait().await.unwrap();
    assert_eq!(ticket.wait().await, Ok(42));
    tokio::time::timeout(Duration::from_secs(3), executor.drain())
        .await
        .unwrap();
}
#[test]
fn dropping_last_executor_cancels_queue_but_running_ticket_can_finish() {
    let executor = executor(1, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let pending = executor.submit(options(0, 0), || Ok::<_, ()>(())).unwrap();
    drop(executor);
    assert_eq!(pending.wait_blocking(), Err(RunError::Cancelled));
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
}
#[test]
fn invalid_configuration_and_indices_are_rejected() {
    assert!(
        Executor::new(Config {
            workers: nz(1),
            priorities: vec![],
            lane_limits: vec![nz(1)]
        })
        .is_err()
    );
    let executor = executor(1, 1);
    let mut opts = options(2, 0);
    assert!(matches!(
        executor.submit(opts, || Ok::<_, ()>(())),
        Err(SubmitError::UnknownPriority)
    ));
    opts = options(0, 2);
    assert!(matches!(
        executor.submit(opts, || Ok::<_, ()>(())),
        Err(SubmitError::UnknownLane)
    ));
    opts = options(0, 0);
    opts.queue_timeout = Some(Duration::MAX);
    assert!(matches!(
        executor.submit(opts, || Ok::<_, ()>(())),
        Err(SubmitError::InvalidDeadline)
    ));
    finish(&executor);
}
#[test]
fn cancelled_closure_destructor_can_reenter_scheduler() {
    struct Reenter(Executor);
    impl Drop for Reenter {
        fn drop(&mut self) {
            let _ = self.0.snapshot();
        }
    }
    let executor = executor(1, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let reenter = Reenter(executor.clone());
    let ticket = executor
        .submit(options(0, 0), move || {
            drop(reenter);
            Ok::<_, ()>(())
        })
        .unwrap();
    drop(ticket);
    assert_eq!(executor.snapshot().queued, vec![0, 0]);
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
    finish(&executor);
}

#[test]
fn timed_out_sqlite_write_can_commit_later_without_automatic_retry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("native.db");
    let reader = rusqlite::Connection::open(&path).unwrap();
    reader
        .execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE writes(value INTEGER);")
        .unwrap();
    let executor = executor(1, 2);
    let (start, started) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut opts = options(0, 0);
    opts.execution_timeout = Some(Duration::from_millis(20));
    let ticket = executor
        .submit(opts, move || {
            let mut connection = rusqlite::Connection::open(path)?;
            let transaction = connection.transaction()?;
            transaction.execute("INSERT INTO writes VALUES(1)", [])?;
            start.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            transaction.commit()
        })
        .unwrap();
    started.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(matches!(
        ticket.wait_blocking(),
        Err(RunError::ExecutionTimeout)
    ));
    assert_eq!(
        reader
            .query_row("SELECT COUNT(*) FROM writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(executor.snapshot().active, 1);
    release.send(()).unwrap();
    finish(&executor);
    assert_eq!(
        reader
            .query_row("SELECT COUNT(*) FROM writes", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn dropped_async_wait_cancels_queued_job_and_running_ticket_drop_keeps_capacity() {
    let executor = executor(1, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let queued = executor.submit(options(0, 0), || Ok::<_, ()>(())).unwrap();
    let mut waiting = Box::pin(queued.wait());
    tokio::select! { biased; _ = &mut waiting => panic!("queued job ran"), _ = tokio::task::yield_now() => {} }
    // The future owns the ticket; dropping it cancels admission before dispatch.
    drop(waiting);
    assert_eq!(executor.snapshot().queued, vec![0, 0]);
    drop(block);
    assert_eq!(executor.snapshot().active, 1);
    release.send(()).unwrap();
    executor.close(CloseMode::CancelQueued);
    tokio::time::timeout(Duration::from_secs(3), executor.drain())
        .await
        .unwrap();
}

#[test]
fn abandoned_result_destructor_panic_does_not_leak_worker_capacity() {
    struct BadDrop;
    impl Drop for BadDrop {
        fn drop(&mut self) {
            panic!("abandoned result destructor");
        }
    }
    let executor = executor(1, 4);
    let (start, started) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let ticket = executor
        .submit(options(0, 0), move || {
            start.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok::<_, ()>(BadDrop)
        })
        .unwrap();
    started.recv_timeout(Duration::from_secs(3)).unwrap();
    drop(ticket);
    release.send(()).unwrap();
    assert_eq!(
        executor
            .submit(options(0, 0), || Ok::<_, ()>(9))
            .unwrap()
            .wait_blocking(),
        Ok(9)
    );
    finish(&executor);
}

#[test]
fn cancelled_capture_destructor_panic_does_not_kill_worker() {
    struct BadDrop;
    impl Drop for BadDrop {
        fn drop(&mut self) {
            panic!("cancelled capture destructor");
        }
    }
    let executor = executor(1, 4);
    let (block, release) = blocker(&executor, 0, 0);
    let capture = BadDrop;
    let ticket = executor
        .submit(options(0, 0), move || {
            drop(capture);
            Ok::<_, ()>(())
        })
        .unwrap();
    drop(ticket);
    assert_eq!(executor.snapshot().queued, vec![0, 0]);
    release.send(()).unwrap();
    block.wait_blocking().unwrap();
    assert_eq!(
        executor
            .submit(options(0, 0), || Ok::<_, ()>(3))
            .unwrap()
            .wait_blocking(),
        Ok(3)
    );
    finish(&executor);
}
