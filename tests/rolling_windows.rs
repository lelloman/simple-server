#![cfg(feature = "rate-limit")]
use simple_server::rate_limit::{
    RollingWindow, RollingWindowError, RollingWindowStore, evaluate_rolling_windows,
};
use std::collections::VecDeque;

struct Events {
    times: VecDeque<Result<i128, &'static str>>,
    events: Vec<(i128, bool)>,
    reads: Vec<(usize, i128)>,
    fail_index: Option<usize>,
}
impl RollingWindowStore for Events {
    type Error = &'static str;
    fn now_micros(&mut self) -> Result<i128, Self::Error> {
        self.times.pop_front().expect("unexpected clock sample")
    }
    fn count_after(&mut self, index: usize, cutoff: i128) -> Result<u64, Self::Error> {
        self.reads.push((index, cutoff));
        if self.fail_index == Some(index) {
            return Err("storage unavailable");
        }
        Ok(self
            .events
            .iter()
            .filter(|(at, success)| *success && *at > cutoff)
            .count() as u64)
    }
}
fn store(times: &[i128]) -> Events {
    Events {
        times: times.iter().copied().map(Ok).collect(),
        events: vec![],
        reads: vec![],
        fail_index: None,
    }
}
fn window(window_micros: u64, limit: u64) -> RollingWindow {
    RollingWindow {
        window_micros,
        limit,
    }
}

#[test]
fn strict_cutoffs_success_accounting_per_window_clock_and_minimum() {
    let mut db = store(&[100, 101, 102]);
    db.events = vec![
        (70, true),
        (80, true),
        (81, false),
        (81, true),
        (100, true),
        (110, true),
    ];
    let result = evaluate_rolling_windows(
        &[window(20, 8), window(31, 5), window(100, 9)],
        100,
        &mut db,
    )
    .unwrap();
    assert_eq!(db.reads, [(0, 80), (1, 70), (2, 2)]);
    assert_eq!(
        result
            .windows
            .iter()
            .map(|w| (w.used, w.remaining))
            .collect::<Vec<_>>(),
        [(3, 5), (4, 1), (5, 4)]
    );
    assert_eq!(result.remaining, 1);
    assert_eq!(db.events.len(), 6); // Never charge or modify history.
}

#[test]
fn empty_configuration_is_the_only_fallback_and_never_reads_storage() {
    let mut db = store(&[]);
    assert_eq!(
        evaluate_rolling_windows(&[], 100, &mut db)
            .unwrap()
            .remaining,
        100
    );
    assert!(db.reads.is_empty());
    let mut db = store(&[0]);
    assert_eq!(
        evaluate_rolling_windows(&[window(1, 500)], 100, &mut db)
            .unwrap()
            .remaining,
        500
    );
}

#[test]
fn zero_duration_zero_limit_excess_usage_and_negative_epoch() {
    let mut db = store(&[-5, -5, -5]);
    db.events = vec![(-6, true), (-5, true), (-4, true)];
    let result =
        evaluate_rolling_windows(&[window(0, 1), window(1, 0), window(2, 1)], 100, &mut db)
            .unwrap();
    assert_eq!(
        result.windows.iter().map(|w| w.used).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(result.windows.iter().all(|w| w.remaining == 0));
    assert_eq!(result.remaining, 0);
}

#[test]
fn errors_propagate_even_after_an_exhausted_window() {
    let mut db = store(&[10, 20, 30]);
    db.fail_index = Some(1);
    assert_eq!(
        evaluate_rolling_windows(&[window(1, 0); 3], 100, &mut db),
        Err(RollingWindowError::Backend("storage unavailable"))
    );
    assert_eq!(db.reads.len(), 2);
    assert_eq!(db.times.len(), 1);
    let mut db = store(&[]);
    db.times.push_back(Err("clock unavailable"));
    assert_eq!(
        evaluate_rolling_windows(&[window(1, 10)], 100, &mut db),
        Err(RollingWindowError::Backend("clock unavailable"))
    );
    assert!(db.reads.is_empty());
    let mut db = store(&[i128::MIN]);
    assert_eq!(
        evaluate_rolling_windows(&[window(1, 10)], 100, &mut db),
        Err(RollingWindowError::TimestampRange { window_index: 0 })
    );
    assert!(db.reads.is_empty());
}

#[test]
fn repeated_evaluations_match_direct_history_model_across_rolling_boundaries() {
    let windows = [window(30, 3), window(120, 8), window(1440, 20)];
    let events: Vec<_> = (-2000..0).step_by(7).map(|t| (t, t % 3 != 0)).collect();
    let mut observed = std::collections::BTreeSet::new();
    for now in (-1500..2000).step_by(11) {
        let mut db = store(&[now; 3]);
        db.events = events
            .iter()
            .copied()
            .filter(|(at, _)| *at <= now)
            .collect();
        let result = evaluate_rolling_windows(&windows, 100, &mut db).unwrap();
        let expected: Vec<_> = windows
            .iter()
            .map(|w| {
                let count = db
                    .events
                    .iter()
                    .filter(|(t, ok)| *ok && *t > now - i128::from(w.window_micros))
                    .count() as u64;
                (count, w.limit.saturating_sub(count))
            })
            .collect();
        assert_eq!(
            result
                .windows
                .iter()
                .map(|w| (w.used, w.remaining))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            result.remaining,
            expected
                .iter()
                .map(|(_, remaining)| *remaining)
                .min()
                .unwrap()
        );
        observed.insert(result.remaining);
    }
    assert!(observed.contains(&0));
    assert!(observed.contains(&3));
    assert!(
        observed.len() > 2,
        "exercise replenishment as history ages out"
    );
}
