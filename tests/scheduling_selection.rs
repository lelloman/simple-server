#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling::{
    BatchReadiness, PriorityAdmissionError, PriorityCapacity, ResourceDemand, WeightedSelection,
    denied_resources,
};
use std::{num::NonZeroUsize, time::Duration};
fn n(v: usize) -> NonZeroUsize {
    NonZeroUsize::new(v).unwrap()
}

#[tokio::test]
async fn weighted_cycle_commits_only_successful_claims_and_preserves_weights() {
    let selection = WeightedSelection::new([(0, n(8)), (1, n(4)), (2, n(2)), (3, n(1))]).unwrap();
    for _ in 0..3 {
        assert_eq!(
            selection
                .claim_next(|_| async { Ok::<Option<usize>, ()>(None) })
                .await,
            Ok(None)
        );
        assert_eq!(
            selection
                .claim_next(|_| async { Err::<Option<usize>, _>(()) })
                .await,
            Err(())
        );
    }
    let mut counts = [0; 4];
    for _ in 0..30 {
        let lane = selection
            .clone()
            .claim_next(|lane| async move { Ok::<_, ()>(Some(lane)) })
            .await
            .unwrap()
            .unwrap();
        counts[lane] += 1;
    }
    assert_eq!(counts, [16, 8, 4, 2]);
    assert_eq!(
        selection
            .claim_next(|lane| async move { Ok::<_, ()>((lane == 3).then_some(lane)) })
            .await,
        Ok(Some(3))
    );
    assert_eq!(
        selection
            .claim_next(|lane| async move { Ok::<_, ()>(Some(lane)) })
            .await,
        Ok(Some(0))
    );
}

#[tokio::test]
async fn cancellation_does_not_advance_weight_cursor() {
    let selection = WeightedSelection::new([(0, n(1)), (1, n(1))]).unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(1),
            selection.claim_next(|_| std::future::pending::<Result<Option<usize>, ()>>())
        )
        .await
        .is_err()
    );
    assert_eq!(
        selection
            .claim_next(|lane| async move { Ok::<_, ()>(Some(lane)) })
            .await,
        Ok(Some(0))
    );
    assert!(WeightedSelection::<u8>::new([]).is_err());
}

#[test]
fn batch_boundaries_and_saturation_are_independent_of_queue_identity() {
    let p = BatchReadiness {
        minimum: 2,
        fill_timeout: Duration::from_millis(50),
        saturation_timeout: Duration::from_millis(500),
    };
    assert!(!p.ready(0, 0, None, false));
    assert!(p.ready(4, 4, None, true));
    assert!(!p.ready(1, 4, Some(Duration::from_secs(5)), false));
    assert!(!p.ready(2, 4, Some(Duration::from_millis(49)), false));
    assert!(p.ready(2, 4, Some(Duration::from_millis(50)), false));
    assert!(!p.ready(2, 4, Some(Duration::from_millis(499)), true));
    assert!(p.ready(2, 4, Some(Duration::from_millis(500)), true));
    assert!(p.ready(1, 0, None, false));
}

#[test]
fn durable_capacity_is_checked_without_overflow_and_reports_all_failures() {
    assert!(
        ResourceDemand {
            used: 3,
            requested: 2,
            limit: 5
        }
        .fits()
    );
    assert_eq!(
        denied_resources([
            (
                "slots",
                ResourceDemand {
                    used: 1,
                    requested: 1,
                    limit: 1
                }
            ),
            (
                "memory",
                ResourceDemand {
                    used: 2,
                    requested: 3,
                    limit: 5
                }
            ),
            (
                "disk",
                ResourceDemand {
                    used: u128::MAX,
                    requested: 1,
                    limit: u128::MAX
                }
            ),
        ]),
        vec!["slots", "disk"]
    );
    assert!(
        !ResourceDemand {
            used: 0,
            requested: 1,
            limit: 0
        }
        .fits()
    );
}

#[tokio::test]
async fn priority_fifo_skips_saturated_classes_and_prunes_cancelled_waiters() {
    let c = PriorityCapacity::new(n(2), vec![2, 2, 1], 3).unwrap();
    let prefetch = c.acquire(2).await.unwrap();
    let active = c.acquire(1).await.unwrap();
    let mut normal = Box::pin(c.acquire(1));
    let mut foreground = Box::pin(c.acquire(0));
    let mut disabled_by_cap = Box::pin(c.acquire(2));
    assert!(futures_util::poll!(&mut normal).is_pending());
    assert!(futures_util::poll!(&mut foreground).is_pending());
    assert!(futures_util::poll!(&mut disabled_by_cap).is_pending());
    assert!(matches!(
        c.acquire(0).await,
        Err(PriorityAdmissionError::QueueFull)
    ));
    drop(disabled_by_cap);
    assert_eq!(c.snapshot().queued, 2);
    drop(active);
    let fg = foreground.await.unwrap();
    assert!(futures_util::poll!(&mut normal).is_pending());
    drop(fg);
    let normal = normal.await.unwrap();
    drop(normal);
    drop(prefetch);
    assert_eq!(c.snapshot().active, 0);
}

#[tokio::test]
async fn cancellation_after_grant_returns_capacity_without_another_enqueue() {
    let c = PriorityCapacity::new(n(1), vec![1], 1).unwrap();
    let active = c.acquire(0).await.unwrap();
    let mut waiting = Box::pin(c.acquire(0));
    assert!(futures_util::poll!(&mut waiting).is_pending());
    drop(active); // Grant into the channel, before its receiver is polled again.
    assert_eq!(c.snapshot().active, 1);
    drop(waiting);
    assert_eq!(c.snapshot().active, 0);
    let _next = c.acquire(0).await.unwrap();
}

#[tokio::test]
async fn zero_class_and_zero_queue_limits_are_explicit() {
    let c = PriorityCapacity::new(n(1), vec![1, 0], 1).unwrap();
    let mut zero = Box::pin(c.acquire(1));
    assert!(futures_util::poll!(&mut zero).is_pending());
    assert!(matches!(
        c.acquire(0).await,
        Err(PriorityAdmissionError::QueueFull)
    ));
    drop(zero);
    let _permit = c.acquire(0).await.unwrap();
    assert!(matches!(
        c.acquire(2).await,
        Err(PriorityAdmissionError::UnknownClass)
    ));
    let c = PriorityCapacity::new(n(1), vec![1], 0).unwrap();
    assert!(matches!(
        c.acquire(0).await,
        Err(PriorityAdmissionError::QueueFull)
    ));
}
