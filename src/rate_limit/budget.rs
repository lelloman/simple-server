use super::token_bucket::ClockRegression;
use std::{fmt, num::NonZeroU32, time::Duration};

/// Invalid policy configuration. Invalid limits are never silently disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigError(pub &'static str);
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ConfigError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Rate,
    Cooldown,
    Concurrency,
    StoreCapacity,
    CostExceedsCapacity,
    ClockRange,
}

/// No key or credential is retained in rejection diagnostics. `retry_after` is
/// absent when passage of time alone cannot give a meaningful retry estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub reason: Reason,
    pub retry_after: Option<Duration>,
}
impl Rejection {
    pub(crate) fn new(reason: Reason, retry_after: Option<Duration>) -> Self {
        Self {
            reason,
            retry_after,
        }
    }
    /// Round upward, with at least one second when a retry duration is known.
    /// Applications may preserve their own historical rounding instead.
    pub fn retry_after_seconds(&self) -> Option<u64> {
        self.retry_after.map(|d| {
            d.as_secs()
                .saturating_add(u64::from(d.subsec_nanos() != 0))
                .max(1)
        })
    }
}
impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "admission rejected: {:?}", self.reason)
    }
}
impl std::error::Error for Rejection {}

#[derive(Clone, Copy, Debug)]
enum Algorithm {
    Replenishing(RefillPolicy),
    FixedWindow,
}

/// Replenishment behavior when a GCRA budget becomes idle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefillPolicy {
    /// Enforce the configured burst even after complete replenishment.
    #[default]
    Strict,
    /// Compatibility with governor 0.6: one interval of initial debt and a full
    /// burst of tolerance. Fresh budgets admit the configured burst; stored
    /// budgets can admit an extra unit after becoming idle. Maximum single-check
    /// cost remains the configured burst.
    ExtraIdleCredit,
}

/// Immutable policy. A replenishing interval restores ONE unit, not an entire
/// burst. Fixed windows start on the first check and reset at `elapsed >= window`.
#[derive(Clone, Copy, Debug)]
pub struct Quota {
    algorithm: Algorithm,
    period: Duration,
    capacity: NonZeroU32,
}
impl Quota {
    pub fn replenishing(interval: Duration, burst: NonZeroU32) -> Result<Self, ConfigError> {
        Self::replenishing_with_policy(interval, burst, RefillPolicy::Strict)
    }
    pub fn replenishing_with_policy(
        interval: Duration,
        burst: NonZeroU32,
        policy: RefillPolicy,
    ) -> Result<Self, ConfigError> {
        if interval.is_zero() || interval.checked_mul(burst.get()).is_none() {
            return Err(ConfigError(
                "refill interval must be positive and burst duration representable",
            ));
        }
        Ok(Self {
            algorithm: Algorithm::Replenishing(policy),
            period: interval,
            capacity: burst,
        })
    }
    pub fn fixed_window(window: Duration, limit: NonZeroU32) -> Result<Self, ConfigError> {
        if window.is_zero() {
            return Err(ConfigError("window must be positive"));
        }
        Ok(Self {
            algorithm: Algorithm::FixedWindow,
            period: window,
            capacity: limit,
        })
    }
    pub fn capacity(&self) -> NonZeroU32 {
        self.capacity
    }
}

/// Small, caller-owned budget usable inside an application lock or transaction.
/// Times must be elapsed monotonic durations from one origin. Backward samples
/// clamp by default; an explicit clock policy can preserve raw observations.
/// Admission is atomic under the caller's exclusive `&mut` access.
#[derive(Clone, Debug)]
pub struct Budget {
    quota: Quota,
    last_now: Duration,
    clock_regression: ClockRegression,
    // GCRA theoretical arrival time or fixed-window start.
    time: Option<Duration>,
    count: u32,
}
impl Budget {
    pub fn new(quota: Quota) -> Self {
        Self {
            quota,
            last_now: Duration::ZERO,
            clock_regression: ClockRegression::default(),
            time: None,
            count: 0,
        }
    }
    /// Select handling of out-of-order caller timestamps before using the budget.
    pub fn with_clock_regression(mut self, policy: ClockRegression) -> Self {
        self.clock_regression = policy;
        self
    }
    pub fn check_at(&mut self, now: Duration, cost: NonZeroU32) -> Result<(), Rejection> {
        if cost.get() > self.quota.capacity.get() {
            return Err(Rejection::new(Reason::CostExceedsCapacity, None));
        }
        let now = match self.clock_regression {
            ClockRegression::Clamp => now.max(self.last_now),
            ClockRegression::Reanchor => now,
        };
        self.last_now = now;
        match self.quota.algorithm {
            Algorithm::Replenishing(policy) => {
                let initial = match policy {
                    RefillPolicy::Strict => now,
                    RefillPolicy::ExtraIdleCredit => now
                        .checked_add(self.quota.period)
                        .ok_or_else(|| Rejection::new(Reason::ClockRange, None))?,
                };
                let tat = self.time.unwrap_or(initial).max(now);
                let weight = self
                    .quota
                    .period
                    .checked_mul(cost.get())
                    .expect("validated capacity");
                let tolerance = self
                    .quota
                    .period
                    .checked_mul(
                        self.quota.capacity.get() - cost.get()
                            + u32::from(policy == RefillPolicy::ExtraIdleCredit),
                    )
                    .expect("validated capacity");
                let earliest = tat.saturating_sub(tolerance);
                if now < earliest {
                    return Err(Rejection::new(Reason::Rate, Some(earliest - now)));
                }
                self.time = Some(
                    tat.checked_add(weight)
                        .ok_or_else(|| Rejection::new(Reason::ClockRange, None))?,
                );
            }
            Algorithm::FixedWindow => {
                let start = self.time.get_or_insert(now);
                if now.saturating_sub(*start) >= self.quota.period {
                    *start = now;
                    self.count = 0;
                }
                if cost.get() > self.quota.capacity.get() - self.count {
                    return Err(Rejection::new(
                        Reason::Rate,
                        Some(self.quota.period.saturating_sub(now.saturating_sub(*start))),
                    ));
                }
                self.count += cost.get();
            }
        }
        Ok(())
    }
    /// True only when discarding this budget cannot restore spent allowance.
    pub fn is_replenished_at(&self, now: Duration) -> bool {
        let now = match self.clock_regression {
            ClockRegression::Clamp => now.max(self.last_now),
            ClockRegression::Reanchor => now,
        };
        self.time.is_none_or(|time| match self.quota.algorithm {
            Algorithm::Replenishing(_) => now >= time,
            Algorithm::FixedWindow => {
                self.count == 0 || now.saturating_sub(time) >= self.quota.period
            }
        })
    }
}

/// Whether outcomes arriving during a cooldown are counted. Counting supports
/// authentication attempts that were admitted concurrently before a block began.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlockedFailures {
    #[default]
    Ignore,
    Count,
}
/// Whether cooldown expiry resets the independent failure window and count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CooldownExpiry {
    #[default]
    Reset,
    PreserveWindow,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct FailurePolicy {
    pub blocked_failures: BlockedFailures,
    pub cooldown_expiry: CooldownExpiry,
}

/// Outcome-driven failure counter. `check_at` never charges an attempt.
/// Defaults ignore blocked failures and reset on cooldown expiry. An explicit
/// policy can count in-flight outcomes during cooldown and retain the independent
/// window after expiry. Success resets only counters selected by the application.
#[derive(Clone, Debug)]
pub struct FailureCounter {
    threshold: NonZeroU32,
    window: Option<Duration>,
    cooldown: Duration,
    policy: FailurePolicy,
    started: Option<Duration>,
    blocked: Option<Duration>,
    failures: u32,
    last_now: Duration,
    clock_regression: ClockRegression,
}
impl FailureCounter {
    pub fn new(
        threshold: NonZeroU32,
        window: Option<Duration>,
        cooldown: Duration,
    ) -> Result<Self, ConfigError> {
        Self::with_policy(threshold, window, cooldown, FailurePolicy::default())
    }
    pub fn with_policy(
        threshold: NonZeroU32,
        window: Option<Duration>,
        cooldown: Duration,
        policy: FailurePolicy,
    ) -> Result<Self, ConfigError> {
        if window.is_some_and(|w| w.is_zero()) || cooldown.is_zero() {
            return Err(ConfigError("failure window and cooldown must be positive"));
        }
        Ok(Self {
            threshold,
            window,
            cooldown,
            policy,
            started: None,
            blocked: None,
            failures: 0,
            last_now: Duration::ZERO,
            clock_regression: ClockRegression::default(),
        })
    }
    /// Preserve raw caller timestamps explicitly when pre-lock observations can
    /// arrive out of order. Defaults retain monotonic clamping.
    pub fn with_clock_regression(mut self, policy: ClockRegression) -> Self {
        self.clock_regression = policy;
        self
    }
    fn observe(&mut self, now: Duration) -> Duration {
        let now = match self.clock_regression {
            ClockRegression::Clamp => now.max(self.last_now),
            ClockRegression::Reanchor => now,
        };
        self.last_now = now;
        now
    }
    pub fn check_at(&mut self, now: Duration) -> Result<(), Rejection> {
        let now = self.observe(now);
        if let Some(until) = self.blocked {
            if now < until {
                return Err(Rejection::new(Reason::Cooldown, Some(until - now)));
            }
            if self.policy.cooldown_expiry == CooldownExpiry::Reset {
                self.reset();
            }
            // PreserveWindow also retains the deadline: an older observation
            // may still see it blocked. Preflight never advances failure windows.
        }
        Ok(())
    }
    /// With `BlockedFailures::Count`, records even while blocked and returns a
    /// cooldown rejection only when this outcome reaches the threshold again.
    /// `Ok` then means no new block was triggered, not that preflight would allow
    /// an attempt. Use `check_at` for preflight under every policy.
    pub fn record_failure_at(&mut self, now: Duration) -> Result<(), Rejection> {
        let now = self.observe(now);
        let preflight = self.check_at(now);
        if self.policy.blocked_failures == BlockedFailures::Ignore {
            preflight?;
        }
        if self
            .window
            .zip(self.started)
            .is_some_and(|(window, start)| now.saturating_sub(start) >= window)
        {
            self.failures = 0;
            self.started = None;
        }
        self.started.get_or_insert(now);
        if self.failures == self.threshold.get() - 1 {
            let until = now
                .checked_add(self.cooldown)
                .ok_or_else(|| Rejection::new(Reason::ClockRange, None))?;
            self.blocked = Some(until);
            self.failures = 0;
            return Err(Rejection::new(Reason::Cooldown, Some(self.cooldown)));
        }
        self.failures += 1;
        Ok(())
    }
    pub fn reset(&mut self) {
        self.failures = 0;
        self.started = None;
        self.blocked = None;
    }
}
