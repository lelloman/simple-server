#![cfg(feature = "rate-limit")]
use simple_server::rate_limit::*;
use std::time::Duration;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn grouped_window_preserves_strict_boundary_usage_and_category_reset() {
    let mut c = WindowCounters::<3>::new(ms(60_000), WindowBoundary::After, ms(100));
    c.admit_at(ms(100), 0, 1, 1).unwrap();
    c.admit_at(ms(200), 1, 2, 1).unwrap();
    assert!(c.admit_at(ms(200), 2, 0, 1).is_err());
    assert_eq!(c.counts(), &[1, 1, 0]);
    assert_eq!(
        c.admit_at(ms(60_100), 0, 1, 1),
        Err(WindowDenial {
            retry_after: Duration::ZERO
        })
    );
    assert_eq!(c.anchor(), ms(100));
    c.admit_at(ms(60_101), 2, 1, 1).unwrap();
    assert_eq!(c.counts(), &[0, 0, 1]);
    assert_eq!(c.anchor(), ms(60_101));
    assert!(c.check_at(ms(0), 2, 1, 1).is_err());
    assert_eq!(c.anchor(), ms(60_101));
}

#[test]
fn idle_loop_ticks_advance_anchor_without_charging_and_outcomes_are_separate() {
    let mut c = WindowCounters::<1>::new(ms(60_000), WindowBoundary::AtOrAfter, ms(10));
    c.check_at(ms(60_010), 0, 2, 1).unwrap();
    assert_eq!(c.anchor(), ms(60_010));
    assert_eq!(c.counts(), &[0]);
    c.check_at(ms(60_020), 0, 2, 1).unwrap();
    c.record(0, 1);
    c.check_at(ms(60_030), 0, 2, 1).unwrap();
    c.record(0, 1);
    assert!(c.check_at(ms(120_009), 0, 2, 1).is_err());
    c.check_at(ms(120_010), 0, 2, 1).unwrap();
    assert_eq!(c.counts(), &[0]);
    assert!(c.check_at(ms(180_010), 0, 0, 1).is_err());
    assert_eq!(c.anchor(), ms(180_010));
}

#[test]
fn grouped_windows_match_legacy_models_for_mixed_ticks_outcomes_and_admissions() {
    for boundary in [WindowBoundary::After, WindowBoundary::AtOrAfter] {
        let mut c = WindowCounters::<3>::new(ms(60), boundary, ms(0));
        let mut anchor = 0_u64;
        let mut counts = [0_u64; 3];
        let mut seed = 7_u64;
        for i in 0..10_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let now = (i / 10) * 50 + seed % 100;
            let elapsed = now.saturating_sub(anchor);
            if match boundary {
                WindowBoundary::After => elapsed > 60,
                WindowBoundary::AtOrAfter => elapsed >= 60,
            } {
                anchor = now;
                counts = [0; 3];
            }
            let index = (seed % 3) as usize;
            let limit = (seed >> 8) % 5;
            let allow = counts[index] < limit;
            assert_eq!(c.check_at(ms(now), index, limit, 1).is_ok(), allow);
            if allow && !seed.is_multiple_of(4) {
                counts[index] += 1;
                c.record(index, 1);
            }
            assert_eq!(c.counts(), &counts);
            assert_eq!(c.anchor(), ms(anchor));
        }
    }
}

#[test]
fn zero_windows_and_counter_overflow_are_explicit() {
    let mut c = WindowCounters::<1>::new(Duration::ZERO, WindowBoundary::After, ms(0));
    c.record(0, u64::MAX);
    c.record(0, 1);
    assert_eq!(c.counts(), &[u64::MAX]);
    assert!(c.admit_at(ms(0), 0, u64::MAX, 1).is_err());
    c.admit_at(ms(1), 0, 1, 1).unwrap();
    assert_eq!(c.counts(), &[1]);
    let mut c = WindowCounters::<1>::new(Duration::ZERO, WindowBoundary::AtOrAfter, ms(1));
    c.admit_at(ms(1), 0, 1, 1).unwrap();
    c.admit_at(ms(1), 0, 1, 1).unwrap();
}

#[test]
fn calendar_gate_keeps_gap_precedence_raw_period_equality_and_restart_state() {
    let day = "2026-09-24".to_owned();
    let next = "2026-09-25".to_owned();
    let mut g = CalendarGate::from_parts(ms(600), 2, None, None, 0);
    g.check_at(ms(100), &day).unwrap();
    g.record_at(ms(100), day.clone());
    assert!(matches!(
        g.check_at(ms(699), &next),
        Err(CalendarDenial::Interval { .. })
    ));
    g.check_at(ms(700), &day).unwrap();
    g.record_at(ms(700), day.clone());
    assert_eq!(g.check_at(ms(1300), &day), Err(CalendarDenial::Quota));
    assert!(matches!(
        g.check_at(ms(0), &day),
        Err(CalendarDenial::Interval { .. })
    ));
    let mut restored = CalendarGate::from_parts(
        ms(600),
        2,
        g.last_admitted(),
        g.period().cloned(),
        g.count(),
    );
    restored.check_at(ms(1300), &next).unwrap();
    assert_eq!(restored.period(), Some(&day)); // preflight never rewrites snapshots
    restored.record_at(ms(1300), next);
    assert_eq!(restored.count(), 1);
    restored.check_at(ms(1900), &day).unwrap();
    restored.record_at(ms(1900), day);
    assert_eq!(restored.count(), 1);
}

#[test]
fn calendar_record_is_independent_and_saturates_late_outcomes() {
    let mut c = CalendarCounter::from_parts(0, Some(7), u32::MAX);
    assert!(!c.allows_at(&8));
    c.record_at(7);
    assert_eq!(c.count(), u32::MAX);
    c.record_at(8);
    assert_eq!(c.count(), 1);
    assert_eq!(c.usage_at(&7), 0);
    let mut g = CalendarGate::from_parts(Duration::ZERO, 0, None, None, 0);
    assert_eq!(g.check_at(ms(0), &7), Err(CalendarDenial::Quota));
    g.record_at(ms(10), 7);
    g.record_at(ms(5), 7);
    assert_eq!(g.last_admitted(), Some(ms(5)));
    assert_eq!(g.count(), 2);
}

#[test]
fn calendar_gate_matches_persisted_legacy_snapshots_and_separate_recording() {
    let (mut last, mut date, mut count) = (None, None, 0_u32);
    let mut gate = CalendarGate::from_parts(ms(600), 24, last, date, count);
    for i in 0..5000_u64 {
        let now = if i % 13 == 0 {
            i.saturating_sub(7) * 300
        } else {
            i * 300
        };
        let day = if i % 17 == 0 { i / 90 } else { i / 100 };
        let expected = !(last.is_some_and(|old: Duration| ms(now).saturating_sub(old) < ms(600))
            || (date == Some(day) && count >= 24));
        assert_eq!(gate.check_at(ms(now), &day).is_ok(), expected);
        if i % 3 != 0 {
            last = Some(ms(now));
            count = if date == Some(day) { count + 1 } else { 1 };
            date = Some(day);
            gate.record_at(ms(now), day);
        }
        assert_eq!(gate.last_admitted(), last);
        assert_eq!(gate.period().copied(), date);
        assert_eq!(gate.count(), count);
    }
}

#[test]
fn snapshot_limits_preserve_signed_values_saturation_and_denial_order() {
    assert!(evaluate_limits(&[LimitCheck::below("negative", -2, -1)]).is_ok());
    assert_eq!(
        evaluate_limits(&[
            LimitCheck::below("quota", 0, 0),
            LimitCheck::projected("capacity", 1, 1, 0, 1)
        ]),
        Err("quota")
    );
    assert_eq!(
        evaluate_limits(&[
            LimitCheck::below("quota", 0, 1),
            LimitCheck::projected("capacity", 1, 1, 0, 1)
        ]),
        Err("capacity")
    );
    assert!(evaluate_limits(&[LimitCheck::projected("exact", 8, 1, 1, 10)]).is_ok());
    assert!(
        evaluate_limits(&[LimitCheck::projected(
            "saturated",
            usize::MAX,
            1,
            1024,
            usize::MAX
        )])
        .is_ok()
    );
    assert_eq!(
        evaluate_limits(&[LimitCheck::projected("reserve", 8, 1, 2, 10)]),
        Err("reserve")
    );
    assert!(evaluate_limits::<()>(&[]).is_ok());
}

#[test]
fn polling_gate_preserves_raw_signed_intervals_and_durable_reconstruction() {
    for interval in [-5, 0, 1, 5, 60] {
        for last in [-100, 0, 100] {
            for now in -110..=170 {
                let mut gate = PollingGate::from_parts(interval, Some(last));
                let allowed = now - last >= interval;
                assert_eq!(gate.admit_at(now).is_ok(), allowed);
                assert_eq!(gate.last_poll_at(), Some(if allowed { now } else { last }));
            }
        }
    }
    let mut fresh = PollingGate::from_parts(5, None);
    fresh.admit_at(0).unwrap();
    assert_eq!(
        fresh.admit_at(4),
        Err(PollingDenial {
            retry_after_seconds: 1
        })
    );
    let mut restored = PollingGate::from_parts(5, fresh.last_poll_at());
    restored.admit_at(5).unwrap();
    assert_eq!(restored.last_poll_at(), Some(5));
    assert!(
        PollingGate::from_parts(i64::MAX, Some(i64::MIN))
            .check_at(i64::MAX)
            .is_ok()
    );
    assert!(
        PollingGate::from_parts(i64::MAX, Some(i64::MAX))
            .check_at(i64::MIN)
            .is_err()
    );
}
