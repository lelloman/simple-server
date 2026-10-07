#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling::ExecutionCapacity;
use std::{collections::BTreeMap, num::NonZeroUsize, time::Duration};
fn n(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
fn capacity(global: usize) -> ExecutionCapacity {
    ExecutionCapacity::new(
        n(global),
        BTreeMap::from([("cpu".into(), n(1)), ("io".into(), n(2))]),
    )
    .unwrap()
}
#[tokio::test]
async fn saturated_pool_leaves_global_slots_for_other_pools() {
    let capacity = capacity(2);
    let cpu = capacity.acquire(Some("cpu")).await.unwrap();
    let pending = capacity.acquire(Some("cpu"));
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut pending)
            .await
            .is_err()
    );
    let io = tokio::time::timeout(Duration::from_secs(1), capacity.acquire(Some("io")))
        .await
        .unwrap()
        .unwrap();
    drop(cpu);
    let cpu = tokio::time::timeout(Duration::from_secs(1), &mut pending)
        .await
        .unwrap()
        .unwrap();
    drop((cpu, io));
}
#[tokio::test]
async fn clones_share_global_limit_even_without_pool() {
    let capacity = capacity(1);
    let clone = capacity.clone();
    let first = capacity.acquire(None).await.unwrap();
    let pending = clone.acquire(Some("io"));
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut pending)
            .await
            .is_err()
    );
    drop(first);
    let _second = tokio::time::timeout(Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn cancelling_wait_for_global_releases_partially_acquired_pool() {
    let capacity = capacity(1);
    let global = capacity.acquire(None).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), capacity.acquire(Some("cpu")))
            .await
            .is_err()
    );
    drop(global);
    let _cpu = tokio::time::timeout(Duration::from_secs(1), capacity.acquire(Some("cpu")))
        .await
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn timeout_request_does_not_release_an_executing_jobs_permit() {
    let capacity = capacity(1);
    let permit = capacity.acquire(Some("cpu")).await.unwrap();
    let (finish, finished) = tokio::sync::oneshot::channel();
    let job = tokio::spawn(async move {
        let _permit = permit;
        finished.await.unwrap();
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), capacity.acquire(Some("io")))
            .await
            .is_err()
    );
    finish.send(()).unwrap();
    job.await.unwrap();
    let _io = tokio::time::timeout(Duration::from_secs(1), capacity.acquire(Some("io")))
        .await
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn unknown_pool_fails_before_waiting_for_global_capacity() {
    let capacity = capacity(1);
    let _occupied = capacity.acquire(None).await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(1), capacity.acquire(Some("missing")))
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("missing"));
}
#[test]
fn invalid_capacity_is_reported_without_panicking() {
    assert!(ExecutionCapacity::new(n(1), BTreeMap::from([(String::new(), n(1))])).is_err());
    assert!(ExecutionCapacity::new(n(usize::MAX), BTreeMap::new()).is_err());
    assert!(ExecutionCapacity::new(n(1), BTreeMap::from([("cpu".into(), n(usize::MAX))])).is_err());
}
