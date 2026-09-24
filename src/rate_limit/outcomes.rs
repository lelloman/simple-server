use super::{Reason, Rejection};
use std::{num::NonZeroU32, time::Duration};

/// Outcome window anchored at the first recorded failure, not the threshold.
/// Preflight never records a failure. Caller-owned storage selects identity,
/// cleanup and success-reset scope. Timestamps must share one monotonic origin;
/// elapsed time saturates for out-of-order observations.
#[derive(Clone, Debug)]
pub struct FailureWindow {
    threshold: u32,
    window: Duration,
    started: Option<Duration>,
    failures: u32,
}
impl FailureWindow {
    /// A zero threshold blocks after the first recorded outcome, while a zero
    /// window is immediately expired and never blocks. These explicit semantics
    /// support legacy configurations without changing application validation.
    pub fn new(threshold: u32, window: Duration) -> Self {
        Self {
            threshold,
            window,
            started: None,
            failures: 0,
        }
    }
    pub fn check_at(&self, now: Duration) -> Result<(), Rejection> {
        if let Some(start) = self.started {
            let elapsed = now.saturating_sub(start);
            if elapsed < self.window && self.failures >= self.threshold {
                return Err(Rejection::new(Reason::Rate, Some(self.window - elapsed)));
            }
        }
        Ok(())
    }
    /// Count an outcome, including an already-in-flight failure during a block.
    /// Expired windows restart here; denials never extend the existing window.
    pub fn record_failure_at(&mut self, now: Duration) {
        if self.is_expired_at(now) {
            self.reset();
        }
        self.started.get_or_insert(now);
        // Only reaching the threshold matters; bounded arithmetic also prevents
        // in-flight failures from overflowing or wrapping an active restriction.
        if self.failures < self.threshold.max(1) {
            self.failures += 1;
        }
    }
    pub fn is_expired_at(&self, now: Duration) -> bool {
        self.started
            .is_some_and(|start| now.saturating_sub(start) >= self.window)
    }
    pub fn reset(&mut self) {
        self.started = None;
        self.failures = 0;
    }
}

/// Failure threshold with a latched cooldown cleared only by explicit reset or
/// dropping the counter. Unlike [`super::FailureCounter`], recording outcomes
/// after expiry never starts a new cooldown until the caller clears the latch.
/// Useful when application preflight owns expiry cleanup and older in-flight
/// outcomes must not extend or restart the lockout.
#[derive(Clone, Debug)]
pub struct FailureLatch {
    threshold: NonZeroU32,
    cooldown: Duration,
    failures: u32,
    blocked: Option<Duration>,
}
impl FailureLatch {
    /// Zero cooldown latches immediately but has no blocking duration. A caller
    /// with legacy zero-threshold behavior can explicitly select threshold one.
    pub fn new(threshold: NonZeroU32, cooldown: Duration) -> Self {
        Self {
            threshold,
            cooldown,
            failures: 0,
            blocked: None,
        }
    }
    /// Does not clear an expired latch. Caller preflight chooses when to reset.
    pub fn check_at(&self, now: Duration) -> Result<(), Rejection> {
        if let Some(until) = self.blocked.filter(|until| now < *until) {
            return Err(Rejection::new(Reason::Cooldown, Some(until - now)));
        }
        Ok(())
    }
    /// Record one failure. A latched counter keeps its deadline, including when
    /// expired. Returns the current block; an unrepresentable deadline returns
    /// ClockRange without consuming the threshold outcome.
    pub fn record_failure_at(&mut self, now: Duration) -> Result<(), Rejection> {
        if self.blocked.is_some() {
            return self.check_at(now);
        }
        if self.failures == self.threshold.get() - 1 {
            self.blocked = Some(
                now.checked_add(self.cooldown)
                    .ok_or_else(|| Rejection::new(Reason::ClockRange, None))?,
            );
        } else {
            self.failures += 1;
        }
        self.check_at(now)
    }
    pub fn is_expired_at(&self, now: Duration) -> bool {
        self.blocked.is_some_and(|until| now >= until)
    }
    pub fn reset(&mut self) {
        self.failures = 0;
        self.blocked = None;
    }
}
