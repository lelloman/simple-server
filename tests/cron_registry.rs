#![cfg(feature = "task-scheduling")]

use simple_server::task_scheduling::{
    CronEntryConfig, CronRegistry, CronRegistryError, CronSchedule, MissedTickPolicy,
};
use std::{
    num::NonZeroUsize,
    time::{Duration, SystemTime},
};

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_735_689_600 + seconds) // 2025-01-01 UTC
}
fn config(expression: &str) -> CronEntryConfig {
    CronEntryConfig::new(CronSchedule::parse(expression).unwrap())
}
fn second() -> CronEntryConfig {
    config("* * * * * *")
}
fn limit(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

#[test]
fn dynamic_changes_are_atomic_and_revisions_detect_stale_notifications() {
    let mut registry = CronRegistry::new();
    let original = registry.register("gc", second(), at(0)).unwrap();
    assert_eq!(
        registry.register("gc", second(), at(20)),
        Err(CronRegistryError::DuplicateId("gc".into()))
    );
    assert_eq!(
        registry.register("", second(), at(0)),
        Err(CronRegistryError::EmptyId)
    );
    assert_eq!(registry.get("gc").unwrap().next_due, Some(at(1)));
    let tick = registry.poll_due(at(1), limit(1)).pop().unwrap();
    assert!(registry.is_current(&tick));
    let replacement = registry
        .replace("gc", config("*/10 * * * * *"), at(1))
        .unwrap();
    assert_ne!(replacement, original);
    assert!(!registry.is_current(&tick));
    assert!(registry.poll_due(at(9), limit(1)).is_empty());
    let tick = registry.poll_due(at(10), limit(1)).pop().unwrap();
    assert_eq!(tick.scheduled_for, at(10));
    assert_eq!(registry.remove("gc").unwrap().revision, replacement);
    assert!(!registry.is_current(&tick));
    let recreated = registry.register("gc", second(), at(10)).unwrap();
    assert_ne!(replacement, recreated);
    assert!(!registry.is_current(&tick));
    assert_eq!(
        registry.replace("missing", second(), at(0)),
        Err(CronRegistryError::UnknownId("missing".into()))
    );
    assert!(matches!(
        registry.remove("missing"),
        Err(CronRegistryError::UnknownId(_))
    ));
    assert_eq!(registry.entries().count(), 1);
}

#[test]
fn disabled_period_is_never_replayed_and_repeated_controls_do_not_reset_timing() {
    let mut registry = CronRegistry::new();
    let mut cfg = second();
    cfg.enabled = false;
    cfg.missed_ticks = MissedTickPolicy::CatchUp;
    registry.register("job", cfg, at(0)).unwrap();
    assert_eq!(registry.next_deadline(), None);
    assert!(registry.poll_due(at(100), limit(10)).is_empty());
    let revision = registry.set_enabled("job", true, at(100)).unwrap();
    assert_eq!(registry.next_deadline(), Some(at(101)));
    assert_eq!(
        registry.set_enabled("job", true, at(200)).unwrap(),
        revision
    );
    assert_eq!(registry.next_deadline(), Some(at(101)));
    let tick = registry.poll_due(at(101), limit(1)).pop().unwrap();
    let disabled = registry.set_enabled("job", false, at(101)).unwrap();
    assert!(!registry.is_current(&tick));
    assert_eq!(
        registry.set_enabled("job", false, at(200)).unwrap(),
        disabled
    );
    registry.set_enabled("job", true, at(300)).unwrap();
    assert_eq!(registry.next_deadline(), Some(at(301)));
    assert!(matches!(
        registry.set_enabled("missing", true, at(0)),
        Err(CronRegistryError::UnknownId(_))
    ));
}

#[test]
fn catch_up_is_bounded_globally_and_orders_deadlines_then_ids() {
    let mut registry = CronRegistry::new();
    let mut cfg = second();
    cfg.missed_ticks = MissedTickPolicy::CatchUp;
    registry.register("b", cfg.clone(), at(0)).unwrap();
    registry.register("a", cfg, at(0)).unwrap();
    let first = registry.poll_due(at(100), limit(3));
    assert_eq!(
        first
            .iter()
            .map(|t| (t.id.as_str(), t.scheduled_for))
            .collect::<Vec<_>>(),
        vec![("a", at(1)), ("b", at(1)), ("a", at(2))]
    );
    let second = registry.poll_due(at(100), limit(3));
    assert_eq!(
        second
            .iter()
            .map(|t| (t.id.as_str(), t.scheduled_for))
            .collect::<Vec<_>>(),
        vec![("b", at(2)), ("a", at(3)), ("b", at(3))]
    );
    assert_eq!(registry.next_deadline(), Some(at(4)));
}

#[test]
fn skip_preserves_undelivered_entries_when_batch_is_full() {
    let mut registry = CronRegistry::new();
    registry.register("a", second(), at(0)).unwrap();
    registry.register("b", second(), at(0)).unwrap();
    assert_eq!(registry.poll_due(at(100), limit(1))[0].id, "a");
    assert_eq!(registry.get("a").unwrap().next_due, Some(at(101)));
    assert_eq!(registry.get("b").unwrap().next_due, Some(at(1)));
    assert_eq!(registry.poll_due(at(100), limit(10))[0].id, "b");
    assert!(registry.poll_due(at(100), limit(10)).is_empty());
    // Rollback must not duplicate a previously delivered occurrence.
    assert!(registry.poll_due(at(0), limit(10)).is_empty());
    assert_eq!(registry.poll_due(at(101), limit(10)).len(), 2);
}

#[test]
fn exact_boundaries_exhausted_years_and_multiple_schedules_are_supported() {
    let mut registry = CronRegistry::new();
    registry
        .register("job/midnight", config("0 0 0 1 1 * 2025"), at(0))
        .unwrap();
    assert_eq!(registry.next_deadline(), None); // Registration is strictly after now.
    registry.register("job/second", second(), at(0)).unwrap();
    registry
        .register("job/tenth", config("*/10 * * * * *"), at(0))
        .unwrap();
    assert!(
        registry
            .poll_due(at(1) - Duration::from_nanos(1), limit(10))
            .is_empty()
    );
    assert_eq!(registry.poll_due(at(1), limit(10)).len(), 1);
    assert_eq!(registry.poll_due(at(10), limit(10)).len(), 2);
    registry
        .register("future", config("0 15 3 1,15 * * 2026-2030"), at(0))
        .unwrap();
    assert_eq!(
        registry.get("future").unwrap().next_due,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_237_300))
    );
}

#[tokio::test]
async fn close_is_irreversible_retains_inspection_and_rejects_mutations() {
    let mut registry = CronRegistry::new();
    registry.register("a", second(), at(0)).unwrap();
    let tick = registry.poll_due(at(1), limit(1)).pop().unwrap();
    registry.close();
    registry.close();
    assert!(registry.is_closed());
    assert_eq!(registry.entries().count(), 1);
    assert!(!registry.is_current(&tick));
    assert_eq!(registry.next_deadline(), None);
    assert!(registry.poll_due(at(100), limit(10)).is_empty());
    assert_eq!(registry.next_due().await, None);
    assert_eq!(
        registry.register("b", second(), at(0)),
        Err(CronRegistryError::Closed)
    );
    assert_eq!(
        registry.replace("a", second(), at(0)),
        Err(CronRegistryError::Closed)
    );
    assert_eq!(
        registry.set_enabled("a", true, at(0)),
        Err(CronRegistryError::Closed)
    );
    assert!(matches!(
        registry.remove("a"),
        Err(CronRegistryError::Closed)
    ));
}

#[tokio::test(start_paused = true)]
async fn interrupted_wait_preserves_cursor_and_allows_live_controls() {
    let mut registry = CronRegistry::new();
    let now = SystemTime::now();
    registry
        .register("job", second(), now + Duration::from_secs(3600))
        .unwrap();
    let deadline = registry.next_deadline();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), registry.next_due())
            .await
            .is_err()
    );
    assert_eq!(registry.next_deadline(), deadline);
    registry.set_enabled("job", false, now).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), registry.next_due())
            .await
            .is_err()
    );
    registry.remove("job").unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), registry.next_due())
            .await
            .is_err()
    );
    registry.register("new", second(), now).unwrap();
    assert!(registry.next_deadline().is_some());
}

#[tokio::test]
async fn real_clock_wait_delivers_ticks_without_owning_or_limiting_execution() {
    let mut registry = CronRegistry::new();
    registry
        .register("job", second(), SystemTime::now())
        .unwrap();
    let tick = tokio::time::timeout(Duration::from_secs(3), registry.next_due())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tick.id, "job");
    assert!(tick.scheduled_for <= SystemTime::now());

    // The caller may start overlapping executions; schedule controls cannot cancel them.
    let work = simple_server::tasks::WorkTracker::default();
    let first = work.try_acquire("cron").unwrap();
    let (release_first, held_first) = tokio::sync::oneshot::channel();
    let first_task = tokio::spawn(async move {
        let _guard = first;
        held_first.await.unwrap();
    });
    registry
        .set_enabled("job", false, SystemTime::now())
        .unwrap();
    let manual = work.try_acquire("manual while cron is disabled").unwrap();
    let (release_manual, held_manual) = tokio::sync::oneshot::channel();
    let manual_task = tokio::spawn(async move {
        let _guard = manual;
        held_manual.await.unwrap();
    });
    registry.close();
    work.close();
    assert_eq!(work.unfinished().len(), 2);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), work.wait())
            .await
            .is_err()
    );
    release_first.send(()).unwrap();
    release_manual.send(()).unwrap();
    work.wait().await;
    first_task.await.unwrap();
    manual_task.await.unwrap();
}

#[tokio::test]
async fn caller_select_loop_serializes_control_against_an_already_due_tick() {
    let mut registry = CronRegistry::new();
    registry
        .register("job", second(), SystemTime::now() - Duration::from_secs(10))
        .unwrap();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    sender.send(()).await.unwrap();
    tokio::select! {
        biased;
        _ = receiver.recv() => { registry.set_enabled("job", false, SystemTime::now()).unwrap(); }
        _ = registry.next_due() => panic!("caller chose control priority"),
    }
    assert!(registry.poll_due(SystemTime::now(), limit(1)).is_empty());
    // If delivery wins first, later removal only invalidates the notification.
    registry.set_enabled("job", true, at(0)).unwrap();
    let tick = registry.next_due().await.unwrap();
    registry.remove("job").unwrap();
    assert!(!registry.is_current(&tick));
}
