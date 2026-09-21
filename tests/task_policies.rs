#![cfg(feature = "task-policies")]
use simple_server::task_policies::*;
use std::{
    num::NonZeroU32,
    time::{Duration, SystemTime},
};
fn retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: NonZeroU32::new(100).unwrap(),
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(10),
        jitter: Duration::from_millis(50),
    }
}
fn circuit() -> CircuitPolicy {
    CircuitPolicy {
        failure_threshold: NonZeroU32::new(2).unwrap(),
        cooldown: Duration::from_secs(30),
    }
}
#[test]
fn retry_attempt_accounting_jitter_caps_and_large_exponents() {
    let policy = retry();
    assert_eq!(policy.delay_after(0, 0.0), None);
    assert_eq!(policy.delay_after(1, 0.5), Some(Duration::from_millis(125)));
    assert_eq!(policy.delay_after(2, 0.0), Some(Duration::from_millis(200)));
    assert_eq!(policy.delay_after(80, 1.0), Some(Duration::from_secs(10)));
    assert_eq!(policy.delay_after(100, 0.0), None);
    let mut tiny = retry();
    tiny.initial_delay = Duration::from_nanos(1);
    assert_eq!(
        tiny.delay_after(33, 0.0),
        Some(Duration::from_nanos(1u64 << 32))
    );
}
#[test]
fn budgets_and_policies_require_valid_nonzero_durations() {
    assert!(
        ExecutionBudget {
            queue_timeout: Some(Duration::ZERO),
            max_runtime: None
        }
        .validate()
        .is_err()
    );
    assert!(ExecutionBudget::default().validate().is_ok());
    let mut invalid = retry();
    invalid.max_delay = Duration::ZERO;
    assert!(invalid.validate().is_err());
    let mut invalid = circuit();
    invalid.cooldown = Duration::ZERO;
    assert!(invalid.validate().is_err());
}
#[test]
fn retry_is_disabled_until_explicitly_classified() {
    let policy = ExecutionPolicy::<&str>::default();
    assert_eq!(policy.retry_delay(&"transient", 1, 0.0), None);
    let policy = policy.with_retry(retry(), |e| *e == "transient");
    assert!(policy.retry_delay(&"transient", 1, 0.0).is_some());
    assert!(policy.retry_delay(&"permanent", 1, 0.0).is_none());
}
#[test]
fn circuit_fences_stale_results_and_allows_exactly_one_probe() {
    let now = SystemTime::UNIX_EPOCH;
    let mut breaker = CircuitBreaker::new(circuit()).unwrap();
    let stale = breaker.admit(now).unwrap();
    for _ in 0..2 {
        let permit = breaker.admit(now).unwrap();
        breaker.record(permit, CircuitOutcome::Failure, now);
    }
    assert!(breaker.admit(now).is_none());
    breaker.record(stale, CircuitOutcome::Success, now);
    assert!(!breaker.can_admit(now));
    let later = now + Duration::from_secs(30);
    let probe = breaker.admit(later).unwrap();
    assert!(breaker.admit(later).is_none());
    breaker.record(probe, CircuitOutcome::Ignored, later);
    let probe = breaker.admit(later).unwrap();
    breaker.record(probe, CircuitOutcome::Failure, later);
    assert!(!breaker.can_admit(later));
    let later = later + Duration::from_secs(30);
    let probe = breaker.admit(later).unwrap();
    breaker.record(probe, CircuitOutcome::Success, later);
    assert_eq!(breaker.snapshot(), CircuitSnapshot::default());
    assert!(breaker.can_admit(later));
}
#[test]
fn circuit_snapshot_restores_controls_but_not_an_in_progress_probe() {
    let now = SystemTime::UNIX_EPOCH;
    let state = CircuitSnapshot {
        consecutive_failures: 2,
        open_until: Some(now),
    };
    let mut original = CircuitBreaker::restore(circuit(), state.clone()).unwrap();
    let _abandoned = original.admit(now).unwrap();
    let mut restored = CircuitBreaker::restore(circuit(), original.snapshot()).unwrap();
    assert!(restored.admit(now).is_some());
    assert!(restored.admit(now).is_none());
    assert!(
        CircuitBreaker::restore(
            circuit(),
            CircuitSnapshot {
                consecutive_failures: 2,
                open_until: None
            }
        )
        .is_err()
    );
}
#[test]
fn pause_scopes_compose_independently() {
    let mut state = PauseState::default();
    state.set(&PauseScope::Pool("cpu".into()), true);
    assert!(state.is_paused("one", Some("cpu")));
    assert!(!state.is_paused("two", Some("io")));
    state.set(&PauseScope::Global, true);
    state.set(&PauseScope::Pool("cpu".into()), false);
    assert!(state.is_paused("one", None));
    state.set(&PauseScope::Global, false);
    state.set(&PauseScope::Job("one".into()), true);
    assert!(state.is_paused("one", None));
    assert!(!state.is_paused("two", None));
}
