#![cfg(feature = "task-policies")]
use simple_server::task_policies::{QuantizedBackoff, doubling_backoff, signed_jitter_millis};
use std::time::Duration;

#[test]
fn floating_backoff_preserves_microsecond_rounding_and_large_attempt_caps() {
    for initial in [0i64, 1, 1234, 1_000_001] {
        for multiplier in [1.0, 1.1, 1.5, 2.0, 10.0] {
            let cap = 1_000_000_000i64;
            let policy = QuantizedBackoff::new(initial as u64, cap as u64, multiplier).unwrap();
            for attempt in (0..70).chain([u32::MAX]) {
                let v = initial as f64
                    * multiplier.powi(i32::try_from(attempt.saturating_sub(1)).unwrap_or(i32::MAX));
                let old = if v.is_finite() {
                    (v as i64).clamp(0, cap)
                } else {
                    cap
                };
                assert_eq!(policy.delay(attempt), old as u64);
            }
        }
    }
    for bad in [f64::NAN, f64::INFINITY, 0.9] {
        assert!(QuantizedBackoff::new(1, 10, bad).is_err());
    }
    assert!(QuantizedBackoff::new(10, 1, 2.0).is_err());
}

#[test]
fn integer_doubling_and_signed_millisecond_jitter_match_legacy_boundaries() {
    for failure in (0u32..70).chain([u32::MAX]) {
        let initial = Duration::new(1, 123456789);
        let cap = Duration::from_secs(1_000_000);
        let old = initial
            .saturating_mul(1 << failure.saturating_sub(1).min(16))
            .min(cap);
        assert_eq!(doubling_backoff(initial, cap, failure, 16), old);
    }
    for base in [
        Duration::ZERO,
        Duration::from_nanos(999_999),
        Duration::from_millis(123),
        Duration::MAX,
    ] {
        for percent in -50i16..=50 {
            let ms = base.as_millis().min(u128::from(u64::MAX)) as u64;
            let delta = ms.saturating_mul(u64::from(percent.unsigned_abs())) / 100;
            let old = if percent < 0 {
                ms.saturating_sub(delta)
            } else {
                ms.saturating_add(delta)
            };
            assert_eq!(
                signed_jitter_millis(base, percent).unwrap(),
                Duration::from_millis(old)
            );
        }
    }
    assert!(signed_jitter_millis(Duration::ZERO, 101).is_err());
    assert_eq!(
        doubling_backoff(Duration::from_secs(1), Duration::MAX, u32::MAX, u32::MAX),
        Duration::MAX
    );
}
