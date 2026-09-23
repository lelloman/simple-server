#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling::{
    PollCadence, PollOutcome, PreferenceOrder, missing_last, run_bounded_batch, run_poll_worker,
};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test(start_paused = true)]
async fn poll_cadence_uses_each_completed_outcome_and_drains_accepted_cycle() {
    let open = Arc::new(AtomicBool::new(true));
    let mut index = 0;
    let mut waited = Vec::new();
    let gate = open.clone();
    run_poll_worker(
        || open.load(Ordering::SeqCst),
        PollCadence {
            worked: Duration::ZERO,
            idle: Duration::from_millis(50),
            failed: Duration::from_millis(100),
        },
        || {
            index += 1;
            let n = index;
            let gate = gate.clone();
            async move {
                match n {
                    1 => PollOutcome::Worked,
                    2 => PollOutcome::Idle,
                    _ => {
                        gate.store(false, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        PollOutcome::Failed
                    }
                }
            }
        },
        |outcome, delay| {
            waited.push((outcome, delay));
            tokio::time::sleep(delay)
        },
    )
    .await;
    assert_eq!(index, 3);
    assert_eq!(
        waited,
        vec![
            (PollOutcome::Idle, Duration::from_millis(50)),
            (PollOutcome::Failed, Duration::from_millis(100))
        ]
    );
}

#[tokio::test]
async fn closed_worker_never_claims() {
    run_poll_worker(
        || false,
        PollCadence {
            worked: Duration::ZERO,
            idle: Duration::ZERO,
            failed: Duration::ZERO,
        },
        || async { panic!("closed admission claimed") },
        |_, _| async {},
    )
    .await;
}

#[tokio::test]
async fn bounded_batch_limits_concurrency_and_observes_panics_without_losing_items() {
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let mut completed = Vec::new();
    let mut panics = 0;
    run_bounded_batch(
        0..9,
        NonZeroUsize::new(2).unwrap(),
        |n| {
            let active = active.clone();
            let peak = peak.clone();
            async move {
                if n == 3 {
                    panic!("fixture");
                }
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                tokio::task::yield_now().await;
                active.fetch_sub(1, Ordering::SeqCst);
                n
            }
        },
        |result| match result {
            Ok(n) => completed.push(n),
            Err(e) => {
                assert!(e.is_panic());
                panics += 1;
            }
        },
    )
    .await;
    completed.sort();
    assert_eq!(completed, vec![0, 1, 2, 4, 5, 6, 7, 8]);
    assert_eq!(panics, 1);
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn dropping_batch_aborts_owned_children() {
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    let (ready_tx, mut ready_rx) = tokio::sync::mpsc::unbounded_channel();
    let count = dropped.clone();
    let task = tokio::spawn(async move {
        run_bounded_batch(
            0..10,
            NonZeroUsize::new(2).unwrap(),
            |_| {
                let guard = Guard(count.clone());
                let ready = ready_tx.clone();
                async move {
                    let _guard = guard;
                    ready.send(()).unwrap();
                    std::future::pending::<()>().await
                }
            },
            |_| {},
        )
        .await
    });
    ready_rx.recv().await.unwrap();
    ready_rx.recv().await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(1), async {
        while dropped.load(Ordering::SeqCst) != 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn preference_and_missing_ranks_have_stable_extreme_boundaries() {
    let order = PreferenceOrder::new(["high".to_string(), "low".into(), "high".into()]);
    assert_eq!(order.position("high"), 2);
    assert_eq!(order.position("low"), 1);
    assert_eq!(order.position("unknown"), usize::MAX);
    let mut values = vec![None, Some(i64::MAX), Some(i64::MIN), Some(0)];
    values.sort_by_key(|v| missing_last(*v));
    assert_eq!(values, vec![Some(i64::MIN), Some(0), Some(i64::MAX), None]);
}
