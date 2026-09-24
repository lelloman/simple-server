use super::{Reason, Rejection};
use std::{num::NonZeroU32, time::Duration};

/// Policy for caller-supplied timestamps observed out of order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClockRegression {
    /// Preserve a monotonic refill anchor.
    #[default]
    Clamp,
    /// Observe zero elapsed on a backward sample, then retain that older anchor.
    /// Later samples refill from it, preserving legacy pre-lock sampling behavior.
    Reanchor,
}

/// Caller-owned floating-point token accounting for services that already use
/// fractional refill. Unlike GCRA, this deliberately retains the arithmetic
/// `elapsed_seconds * per_minute / 60.0` and rounds retry times to whole seconds.
/// Store capacity, synchronization, concurrency and cleanup belong to the caller.
#[derive(Clone, Debug)]
pub struct TokenBucket {
    per_minute: NonZeroU32,
    burst: NonZeroU32,
    tokens: f64,
    last_refill: Duration,
    clock_regression: ClockRegression,
}
impl TokenBucket {
    /// Starts full. Times passed to this bucket must share one monotonic origin.
    pub fn per_minute(rate: NonZeroU32, burst: NonZeroU32) -> Self {
        Self {
            per_minute: rate,
            burst,
            tokens: f64::from(burst.get()),
            last_refill: Duration::ZERO,
            clock_regression: ClockRegression::default(),
        }
    }

    /// Select timestamp behavior before using this bucket. Reanchoring is an
    /// explicit compatibility choice for observations delivered out of order.
    pub fn with_clock_regression(mut self, policy: ClockRegression) -> Self {
        self.clock_regression = policy;
        self
    }

    /// Observe elapsed time without charging. This permits a service to refill
    /// before its concurrency gate, including on attempts denied by that gate.
    /// Backward observations follow the explicitly selected clock policy.
    pub fn refill_at(&mut self, now: Duration) {
        let now = match self.clock_regression {
            ClockRegression::Clamp => now.max(self.last_refill),
            ClockRegression::Reanchor => now,
        };
        let elapsed = now.saturating_sub(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * f64::from(self.per_minute.get()) / 60.0)
            .min(f64::from(self.burst.get()));
        self.last_refill = now;
    }

    /// Refill then consume a positive cost if available. Denial never charges.
    /// Retry durations are rounded upward to whole seconds, with minimum one.
    /// Calling this immediately after `refill_at` with the same time is valid.
    pub fn check_at(&mut self, now: Duration, cost: NonZeroU32) -> Result<(), Rejection> {
        if cost.get() > self.burst.get() {
            return Err(Rejection::new(Reason::CostExceedsCapacity, None));
        }
        self.refill_at(now);
        let cost = f64::from(cost.get());
        if self.tokens < cost {
            let seconds = ((cost - self.tokens) * 60.0 / f64::from(self.per_minute.get())).ceil();
            return Err(Rejection::new(
                Reason::Rate,
                Some(Duration::from_secs_f64(seconds.max(1.0))),
            ));
        }
        self.tokens -= cost;
        Ok(())
    }
}
