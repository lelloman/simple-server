use super::{Reason, Rejection};
use std::{num::NonZeroU32, time::Duration};

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
}
impl TokenBucket {
    /// Starts full. Times passed to this bucket must share one monotonic origin.
    pub fn per_minute(rate: NonZeroU32, burst: NonZeroU32) -> Self {
        Self {
            per_minute: rate,
            burst,
            tokens: f64::from(burst.get()),
            last_refill: Duration::ZERO,
        }
    }

    /// Observe elapsed time without charging. This permits a service to refill
    /// before its concurrency gate, including on attempts denied by that gate.
    /// Backward observations clamp to the last sample.
    pub fn refill_at(&mut self, now: Duration) {
        let now = now.max(self.last_refill);
        let elapsed = (now - self.last_refill).as_secs_f64();
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
