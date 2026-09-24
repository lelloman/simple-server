use super::{Reason, Rejection};
use std::time::Duration;

/// Caller-owned compatibility accounting for existing per-second float buckets.
///
/// Preserves `min(tokens + elapsed_seconds * rate, burst)` exactly, including
/// zero capacity and unvalidated negative/nonfinite rates. This is an explicit
/// legacy policy, not configuration validation: applications adopting new limits
/// should normally use [`super::Quota`]. Storage and synchronization stay local.
#[derive(Clone, Debug)]
pub struct PerSecondTokenBucket {
    rate: f64,
    burst: u32,
    tokens: f64,
    last_observed: Duration,
}

impl PerSecondTokenBucket {
    /// Start full at a caller-supplied observation. All timestamps must share
    /// one origin. Separate construction and admission observations are allowed.
    pub fn new(rate: f64, burst: u32, now: Duration) -> Self {
        Self {
            rate,
            burst,
            tokens: f64::from(burst),
            last_observed: now,
        }
    }

    /// Last admission observation, including denied requests. Callers may use
    /// this for their existing idle eviction policy, regardless of token debt.
    pub fn last_observed(&self) -> Duration {
        self.last_observed
    }

    /// Refill and attempt to consume one unit. Denials do not charge.
    ///
    /// Elapsed time saturates on backward observations, then the anchor is
    /// replaced with that observation. Positive rates report ceiling seconds
    /// using Rust's saturating float-to-u64 conversion; other rates report 60s.
    /// Consequently a retry can be zero or u64::MAX. Use the returned duration
    /// directly when preserving that contract: the generic HTTP renderer applies
    /// its own minimum-one-second policy.
    pub fn check_at(&mut self, now: Duration) -> Result<(), Rejection> {
        let elapsed = now.saturating_sub(self.last_observed).as_secs_f64();
        self.last_observed = now;
        self.tokens = (self.tokens + elapsed * self.rate).min(f64::from(self.burst));
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            let seconds = if self.rate > 0.0 {
                ((1.0 - self.tokens) / self.rate).ceil() as u64
            } else {
                60
            };
            Err(Rejection::new(
                Reason::Rate,
                Some(Duration::from_secs(seconds)),
            ))
        }
    }
}
