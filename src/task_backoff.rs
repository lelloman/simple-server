//! Backoff primitives for applications retaining attempt state and durable clocks.
use crate::task_policies::PolicyError;
use std::time::Duration;

/// Floating-multiplier exponential delay in caller-defined integer ticks (e.g.
/// microseconds). Truncates fractional ticks toward zero, then caps. This does not
/// decide exhaustion, classify errors, sleep, or advance any persistent deadline.
#[derive(Clone, Copy, Debug)]
pub struct QuantizedBackoff {
    initial: u64,
    maximum: u64,
    multiplier: f64,
}
impl QuantizedBackoff {
    pub fn new(initial: u64, maximum: u64, multiplier: f64) -> Result<Self, PolicyError> {
        if initial > maximum || !multiplier.is_finite() || multiplier < 1.0 {
            return Err(PolicyError(
                "backoff requires ordered bounds and a finite multiplier >= 1",
            ));
        }
        Ok(Self {
            initial,
            maximum,
            multiplier,
        })
    }
    /// Attempt zero and one both use the initial delay; attempts saturate safely.
    pub fn delay(self, attempt: u32) -> u64 {
        let exponent = i32::try_from(attempt.saturating_sub(1)).unwrap_or(i32::MAX);
        let calculated = self.initial as f64 * self.multiplier.powi(exponent);
        if calculated.is_finite() {
            (calculated as u64).min(self.maximum)
        } else {
            self.maximum
        }
    }
}

/// Integer doubling without floating conversion or an Instant representability
/// restriction. `exponent_limit` preserves products' historical saturation rules.
/// The returned delay does not include jitter; applying jitter after the cap is
/// an explicit caller choice.
pub fn doubling_backoff(
    initial: Duration,
    maximum: Duration,
    failure: u32,
    exponent_limit: u32,
) -> Duration {
    let exponent = failure.saturating_sub(1).min(exponent_limit);
    let nanos = 1u128
        .checked_shl(exponent)
        .and_then(|factor| initial.as_nanos().checked_mul(factor))
        .unwrap_or(u128::MAX)
        .min(maximum.as_nanos());
    Duration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    )
}

/// Apply a caller-sampled signed percentage in whole milliseconds. The base is
/// truncated and clamped to u64 milliseconds; multiply/add/subtract saturate.
/// Randomness remains caller-owned. No post-jitter cap is silently applied.
pub fn signed_jitter_millis(base: Duration, percent: i16) -> Result<Duration, PolicyError> {
    if !(-100..=100).contains(&percent) {
        return Err(PolicyError("jitter percentage must be in -100..=100"));
    }
    let millis = base.as_millis().min(u128::from(u64::MAX)) as u64;
    let delta = millis.saturating_mul(u64::from(percent.unsigned_abs())) / 100;
    Ok(Duration::from_millis(if percent < 0 {
        millis.saturating_sub(delta)
    } else {
        millis.saturating_add(delta)
    }))
}
