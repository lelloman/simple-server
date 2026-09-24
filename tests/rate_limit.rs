#![cfg(feature = "rate-limit")]
use simple_server::rate_limit::*;
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::{Arc, Barrier, Mutex},
    time::Duration,
};
fn n(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}
fn count(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn sec(n: u64) -> Duration {
    Duration::from_secs(n)
}
#[derive(Clone, Default)]
struct ManualClock(Arc<Mutex<Duration>>);
impl ManualClock {
    fn set(&self, t: Duration) {
        *self.0.lock().unwrap() = t;
    }
}
impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.0.lock().unwrap()
    }
}
fn limiter(clock: ManualClock, burst: u32, capacity: usize) -> KeyedLimiter<&'static str> {
    KeyedLimiter::with_clock(
        Quota::replenishing(sec(10), n(burst)).unwrap(),
        StoreConfig::bounded(count(capacity), sec(1)),
        clock,
    )
}

#[test]
fn validates_policy_and_retry_rounding_without_keys() {
    assert!(Quota::replenishing(Duration::ZERO, n(1)).is_err());
    assert!(Quota::replenishing(Duration::MAX, n(2)).is_err());
    assert!(Quota::fixed_window(Duration::ZERO, n(1)).is_err());
    assert!(FailureCounter::new(n(1), Some(Duration::ZERO), sec(1)).is_err());
    assert!(FailureCounter::new(n(1), None, Duration::ZERO).is_err());
    let r = Rejection {
        reason: Reason::Rate,
        retry_after: Some(Duration::from_millis(1001)),
    };
    assert_eq!(r.retry_after_seconds(), Some(2));
    let r = Rejection {
        reason: Reason::Concurrency,
        retry_after: None,
    };
    assert_eq!(r.retry_after_seconds(), None);
    assert_eq!(rejection_response::<String>(r).status(), 429);
    assert!(
        !rejection_response::<String>(r)
            .headers()
            .contains_key("retry-after")
    );
    assert_eq!(
        rejection_response::<String>(Rejection {
            reason: Reason::StoreCapacity,
            retry_after: None
        })
        .status(),
        503
    );
}

#[test]
fn burst_refill_weight_and_backward_clock_boundaries() {
    let mut b = Budget::new(Quota::replenishing(sec(10), n(3)).unwrap());
    assert!(b.check_at(sec(50), n(4)).is_err()); // Does not charge.
    assert!(b.check_at(sec(50), n(3)).is_ok());
    assert_eq!(
        b.check_at(sec(59), n(1)).unwrap_err().retry_after,
        Some(sec(1))
    );
    assert!(b.check_at(sec(60), n(1)).is_ok());
    assert_eq!(
        b.check_at(sec(20), n(1)).unwrap_err().retry_after,
        Some(sec(10))
    );
    assert!(!b.is_replenished_at(sec(89)));
    assert!(b.is_replenished_at(sec(90)));
    assert!(b.check_at(sec(90), n(3)).is_ok());
    assert_eq!(b.check_at(sec(90), n(1)).unwrap_err().reason, Reason::Rate);
}

#[test]
fn clock_overflow_is_explicit_not_a_free_budget() {
    let mut b = Budget::new(Quota::replenishing(sec(1), n(1)).unwrap());
    assert_eq!(
        b.check_at(Duration::MAX, n(1)).unwrap_err().reason,
        Reason::ClockRange
    );
}

#[test]
fn fixed_window_resets_at_boundary_not_on_rejection() {
    let mut b = Budget::new(Quota::fixed_window(sec(60), n(3)).unwrap());
    b.check_at(sec(10), n(2)).unwrap();
    assert_eq!(
        b.check_at(sec(69), n(2)).unwrap_err().retry_after,
        Some(sec(1))
    );
    b.check_at(sec(69), n(1)).unwrap();
    assert!(!b.is_replenished_at(sec(69)));
    b.check_at(sec(70), n(3)).unwrap();
    assert_eq!(
        b.check_at(sec(70), n(1)).unwrap_err().retry_after,
        Some(sec(60))
    );
}

#[test]
fn failure_cooldown_is_outcome_driven_and_success_reset_is_explicit() {
    let mut pair = FailureCounter::new(n(2), Some(sec(10)), sec(20)).unwrap();
    let mut peer = FailureCounter::new(n(3), Some(sec(10)), sec(20)).unwrap();
    for _ in 0..100 {
        pair.check_at(sec(0)).unwrap();
    }
    pair.record_failure_at(sec(0)).unwrap();
    peer.record_failure_at(sec(0)).unwrap();
    assert_eq!(
        pair.record_failure_at(sec(1)).unwrap_err().retry_after,
        Some(sec(20))
    );
    assert_eq!(
        pair.record_failure_at(sec(5)).unwrap_err().retry_after,
        Some(sec(16))
    ); // no extension
    pair.check_at(sec(21)).unwrap();
    pair.record_failure_at(sec(21)).unwrap();
    pair.record_failure_at(sec(31)).unwrap(); // expired count window
    peer.record_failure_at(sec(1)).unwrap();
    pair.reset(); // successful login clears pair, NOT the peer's spraying counter
    assert_eq!(
        peer.record_failure_at(sec(2)).unwrap_err().reason,
        Reason::Cooldown
    );
    pair.check_at(sec(31)).unwrap();
    let mut account = FailureCounter::new(n(2), None, sec(30)).unwrap();
    account.record_failure_at(sec(0)).unwrap();
    assert!(account.record_failure_at(sec(1000)).is_err()); // no failure window
}

#[test]
fn capacity_cleanup_never_restores_spent_budget_or_removes_live_permits() {
    let clock = ManualClock::default();
    let l = limiter(clock.clone(), 2, 1);
    let permit = l.acquire("a", n(2), Some(count(1))).unwrap();
    clock.set(sec(1));
    assert_eq!(l.prune(), 0);
    assert_eq!(
        l.check("b", n(1)).unwrap_err().reason,
        Reason::StoreCapacity
    );
    clock.set(sec(21));
    assert_eq!(l.prune(), 0); // full, but active
    drop(permit);
    assert_eq!(l.prune(), 1);
    l.check("b", n(1)).unwrap();
    assert_eq!(l.stats().entries, 1);
    assert_eq!(l.stats().evictions, 1);
}

#[test]
fn bounded_store_uses_explicit_shared_overflow_not_eviction() {
    let clock = ManualClock::default();
    let l = KeyedLimiter::with_clock(
        Quota::replenishing(sec(10), n(1)).unwrap(),
        StoreConfig::bounded(count(1), sec(60)).overflow(Overflow::SharedBucket),
        clock,
    );
    l.check("known", n(1)).unwrap();
    l.check("new-a", n(1)).unwrap();
    assert_eq!(l.check("new-b", n(1)).unwrap_err().reason, Reason::Rate);
    assert_eq!(l.stats().entries, 1);
    assert_eq!(l.stats().overflow_checks, 2);
    assert!(l.check("known", n(1)).is_err());
}

#[test]
fn concurrency_rejection_does_not_charge_and_drop_does_not_refund_rate() {
    let clock = ManualClock::default();
    let l = limiter(clock, 2, 2);
    let a = l.acquire("key", n(1), Some(count(1))).unwrap();
    assert_eq!(
        l.acquire("key", n(1), Some(count(1))).unwrap_err().reason,
        Reason::Concurrency
    );
    drop(a);
    let b = l.acquire("key", n(1), Some(count(1))).unwrap();
    drop(b);
    assert_eq!(
        l.acquire("key", n(1), Some(count(1))).unwrap_err().reason,
        Reason::Rate
    );
    assert_eq!(l.stats().active, 0);
}

#[test]
fn simultaneous_admission_cannot_overspend_shared_clones() {
    let l = limiter(ManualClock::default(), 7, 1);
    let start = Arc::new(Barrier::new(33));
    let workers: Vec<_> = (0..32)
        .map(|_| {
            let l = l.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                l.check("same", n(1)).is_ok()
            })
        })
        .collect();
    start.wait();
    assert_eq!(
        workers
            .into_iter()
            .map(|t| usize::from(t.join().unwrap()))
            .sum::<usize>(),
        7
    );
}

#[test]
fn ordered_policies_preserve_prior_charge_but_release_permits_on_denial() {
    let l = limiter(ManualClock::default(), 2, 1);
    let captured = l.clone();
    let p = Policy::new(move |_: &()| captured.acquire("global", n(1), Some(count(1)))).with_check(
        |_| {
            Err(Rejection {
                reason: Reason::Rate,
                retry_after: Some(sec(1)),
            })
        },
    );
    assert!(p.evaluate(&()).is_err());
    assert_eq!(l.stats().active, 0);
    l.check("global", n(1)).unwrap();
    assert!(l.check("global", n(1)).is_err());
}

#[tokio::test]
async fn async_policy_cancellation_releases_guard_and_does_not_spawn_or_refund() {
    let l = limiter(ManualClock::default(), 1, 1);
    let captured = l.clone();
    let policy: AsyncPolicy<(), Rejection> = AsyncPolicy::from_sync(Policy::new(move |_: &()| {
        captured.acquire("x", n(1), Some(count(1)))
    }))
    .with_check(|_| Box::pin(std::future::pending()));
    let mut future = Box::pin(policy.evaluate(&()));
    assert!(futures_util::poll!(future.as_mut()).is_pending());
    assert_eq!(l.stats().active, 1);
    drop(future);
    assert_eq!(l.stats().active, 0);
    assert!(l.check("x", n(1)).is_err());
}

#[test]
fn gcra_matches_both_consumer_governor_versions_for_weighted_sequences() {
    use governor::clock::Clock as _;
    use governor_08::clock::Clock as _;
    for (interval, burst) in [
        (Duration::from_nanos(13), 7),
        (Duration::from_millis(600), 100),
        (sec(36), 10),
    ] {
        let c10 = governor::clock::FakeRelativeClock::default();
        let c08 = governor_08::clock::FakeRelativeClock::default();
        let g10 = governor::RateLimiter::direct_with_clock(
            governor::Quota::with_period(interval)
                .unwrap()
                .allow_burst(n(burst)),
            c10.clone(),
        );
        let g08 = governor_08::RateLimiter::direct_with_clock(
            governor_08::Quota::with_period(interval)
                .unwrap()
                .allow_burst(n(burst)),
            c08.clone(),
        );
        let mut b = Budget::new(Quota::replenishing(interval, n(burst)).unwrap());
        let mut now = Duration::ZERO;
        for i in 0..2000u32 {
            let advance = if i % 4 == 0 {
                interval / 3
            } else {
                Duration::ZERO
            };
            now += advance;
            c10.advance(advance);
            c08.advance(advance);
            let cost = n(1 + (i % burst));
            let result = b.check_at(now, cost);
            let a = g10.check_n(cost).unwrap();
            let old = g08.check_n(cost).unwrap();
            assert_eq!(result.is_ok(), a.is_ok(), "{i}");
            assert_eq!(result.is_ok(), old.is_ok());
            if let Err(r) = result {
                assert_eq!(
                    r.retry_after.unwrap(),
                    a.unwrap_err().wait_time_from(c10.now())
                );
                assert_eq!(
                    r.retry_after.unwrap(),
                    old.unwrap_err().wait_time_from(c08.now())
                );
            }
        }
    }
}

#[test]
fn fractional_bucket_preserves_legacy_refill_and_retry_across_denied_attempts() {
    // Independent reference copied from the gateway's established arithmetic.
    for (per_minute, burst) in [(6, 12), (12, 20), (30, 30), (120, 120), (600, 48)] {
        let mut shared = TokenBucket::per_minute(n(per_minute), n(burst));
        let mut tokens = f64::from(burst);
        let mut last = Duration::ZERO;
        let mut now = Duration::ZERO;
        for i in 0..10_000_u32 {
            now += Duration::from_nanos(u64::from((i * 7_919) % 1_000_003));
            if i % 71 == 0 {
                now += sec(1);
            }
            tokens = (tokens + (now - last).as_secs_f64() * f64::from(per_minute) / 60.0)
                .min(f64::from(burst));
            last = now;
            // Concurrency denial still refills, but must never spend tokens.
            shared.refill_at(now);
            if i % 7 == 0 {
                continue;
            }
            let expected = if tokens < 1.0 {
                Some(
                    ((1.0 - tokens) * 60.0 / f64::from(per_minute))
                        .ceil()
                        .max(1.0) as u64,
                )
            } else {
                tokens -= 1.0;
                None
            };
            let actual = shared
                .check_at(now, n(1))
                .err()
                .map(|e| e.retry_after.unwrap().as_secs());
            assert_eq!(
                actual, expected,
                "rate={per_minute}, burst={burst}, attempt={i}"
            );
        }
    }
}

#[test]
fn fractional_bucket_handles_weight_capacity_refill_and_backward_time() {
    let mut bucket = TokenBucket::per_minute(n(60), n(3));
    assert_eq!(
        bucket.check_at(sec(0), n(4)).unwrap_err().reason,
        Reason::CostExceedsCapacity
    );
    bucket.check_at(sec(0), n(3)).unwrap();
    assert_eq!(
        bucket
            .check_at(Duration::from_millis(500), n(1))
            .unwrap_err()
            .retry_after,
        Some(sec(1))
    );
    assert!(bucket.check_at(sec(0), n(1)).is_err());
    bucket.check_at(sec(1), n(1)).unwrap();
    bucket.refill_at(sec(100));
    bucket.check_at(sec(100), n(3)).unwrap();
    assert!(bucket.check_at(sec(100), n(1)).is_err());
}

#[test]
fn independent_failure_window_matches_existing_outcome_policy() {
    // Model the old Crumbles counter, including outcomes arriving while blocked.
    for (threshold, window, cooldown) in [(2, 10, 20), (3, 60, 2), (1, 7, 7)] {
        let mut shared = FailureCounter::with_policy(
            n(threshold),
            Some(sec(window)),
            sec(cooldown),
            FailurePolicy {
                blocked_failures: BlockedFailures::Count,
                cooldown_expiry: CooldownExpiry::PreserveWindow,
            },
        )
        .unwrap();
        let mut start = None;
        let mut failures = 0_u32;
        let mut blocked = None;
        let mut now = Duration::ZERO;
        for i in 0..5_000_u64 {
            now += Duration::from_millis((i * 137) % 3_000);
            if i % 3 != 0 {
                let start = start.get_or_insert(now);
                if now - *start >= sec(window) {
                    *start = now;
                    failures = 0;
                }
                failures += 1;
                let triggered = failures >= threshold;
                if triggered {
                    failures = 0;
                    blocked = Some(now + sec(cooldown));
                }
                assert_eq!(shared.record_failure_at(now).is_err(), triggered);
            }
            let expected = blocked
                .and_then(|until: Duration| until.checked_sub(now))
                .filter(|d| !d.is_zero());
            let actual = shared.check_at(now).err().and_then(|e| e.retry_after);
            assert_eq!(actual, expected, "threshold={threshold}, step={i}");
        }
    }
}

#[test]
fn counted_outcomes_extend_blocks_and_preserve_counts_after_expiry() {
    let mut counter = FailureCounter::with_policy(
        n(2),
        Some(sec(100)),
        sec(10),
        FailurePolicy {
            blocked_failures: BlockedFailures::Count,
            cooldown_expiry: CooldownExpiry::PreserveWindow,
        },
    )
    .unwrap();
    counter.record_failure_at(sec(0)).unwrap();
    assert!(counter.record_failure_at(sec(1)).is_err());
    counter.record_failure_at(sec(2)).unwrap();
    assert_eq!(
        counter.check_at(sec(2)).unwrap_err().retry_after,
        Some(sec(9))
    );
    assert!(counter.record_failure_at(sec(3)).is_err());
    counter.record_failure_at(sec(4)).unwrap();
    counter.check_at(sec(13)).unwrap();
    assert!(counter.record_failure_at(sec(14)).is_err());
    counter.reset();
    counter.check_at(sec(14)).unwrap();
    counter.record_failure_at(sec(14)).unwrap();
}

#[test]
fn fractional_bucket_can_preserve_out_of_order_observations() {
    let mut legacy =
        TokenBucket::per_minute(n(60), n(1)).with_clock_regression(ClockRegression::Reanchor);
    let mut monotonic = TokenBucket::per_minute(n(60), n(1));
    for bucket in [&mut legacy, &mut monotonic] {
        bucket.check_at(sec(10), n(1)).unwrap();
        assert!(bucket.check_at(sec(9), n(1)).is_err());
    }
    // Legacy behavior anchors at 9 after the backward observation; default
    // behavior retains 10. A later observation exposes that deliberate choice.
    legacy.check_at(sec(10), n(1)).unwrap();
    assert!(monotonic.check_at(sec(10), n(1)).is_err());

    let mut shared =
        TokenBucket::per_minute(n(120), n(8)).with_clock_regression(ClockRegression::Reanchor);
    let mut tokens = 8.0_f64;
    let mut last = Duration::ZERO;
    for i in 0..20_000_u64 {
        let now = Duration::from_millis(i * 37).saturating_sub(if i % 13 == 0 {
            Duration::from_millis(500)
        } else {
            Duration::ZERO
        });
        tokens = (tokens + now.saturating_sub(last).as_secs_f64() * 120.0 / 60.0).min(8.0);
        last = now;
        shared.refill_at(now);
        if i % 7 == 0 {
            continue;
        } // Concurrency gate rejects after observing time.
        let expected = if tokens < 1.0 {
            Some(((1.0 - tokens) * 60.0 / 120.0).ceil().max(1.0) as u64)
        } else {
            tokens -= 1.0;
            None
        };
        assert_eq!(
            shared
                .check_at(now, n(1))
                .err()
                .and_then(|e| e.retry_after_seconds()),
            expected
        );
    }
}
