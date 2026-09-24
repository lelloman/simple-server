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
    Replenishing,
    FixedWindow,
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
        if interval.is_zero() || interval.checked_mul(burst.get()).is_none() {
            return Err(ConfigError(
                "refill interval must be positive and burst duration representable",
            ));
        }
        Ok(Self {
            algorithm: Algorithm::Replenishing,
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
/// are clamped. An admission is atomic under the caller's exclusive `&mut` access.
#[derive(Clone, Debug)]
pub struct Budget {
    quota: Quota,
    last_now: Duration,
    // GCRA theoretical arrival time or fixed-window start.
    time: Option<Duration>,
    count: u32,
}
impl Budget {
    pub fn new(quota: Quota) -> Self {
        Self {
            quota,
            last_now: Duration::ZERO,
            time: None,
            count: 0,
        }
    }
    pub fn check_at(&mut self, now: Duration, cost: NonZeroU32) -> Result<(), Rejection> {
        if cost.get() > self.quota.capacity.get() {
            return Err(Rejection::new(Reason::CostExceedsCapacity, None));
        }
        let now = now.max(self.last_now);
        self.last_now = now;
        match self.quota.algorithm {
            Algorithm::Replenishing => {
                let tat = self.time.unwrap_or(now).max(now);
                let weight = self
                    .quota
                    .period
                    .checked_mul(cost.get())
                    .expect("validated capacity");
                let tolerance = self
                    .quota
                    .period
                    .checked_mul(self.quota.capacity.get() - cost.get())
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
                        Some(self.quota.period.saturating_sub(now - *start)),
                    ));
                }
                self.count += cost.get();
            }
        }
        Ok(())
    }
    /// True only when discarding this budget cannot restore spent allowance.
    pub fn is_replenished_at(&self, now: Duration) -> bool {
        let now = now.max(self.last_now);
        self.time.is_none_or(|time| match self.quota.algorithm {
            Algorithm::Replenishing => now >= time,
            Algorithm::FixedWindow => {
                self.count == 0 || now.saturating_sub(time) >= self.quota.period
            }
        })
    }
}

/// Outcome-driven failure counter. `check_at` does not charge an attempt.
/// `record_failure_at` blocks at the threshold; blocked failures do not extend
/// the cooldown. Successful authentication resets only the counters explicitly
/// selected by the application. `window = None` accumulates until reset/cooldown.
#[derive(Clone, Debug)]
pub struct FailureCounter {
    threshold: NonZeroU32,
    window: Option<Duration>,
    cooldown: Duration,
    started: Option<Duration>,
    blocked: Option<Duration>,
    failures: u32,
    last_now: Duration,
}
impl FailureCounter {
    pub fn new(
        threshold: NonZeroU32,
        window: Option<Duration>,
        cooldown: Duration,
    ) -> Result<Self, ConfigError> {
        if window.is_some_and(|w| w.is_zero()) || cooldown.is_zero() {
            return Err(ConfigError("failure window and cooldown must be positive"));
        }
        Ok(Self {
            threshold,
            window,
            cooldown,
            started: None,
            blocked: None,
            failures: 0,
            last_now: Duration::ZERO,
        })
    }
    pub fn check_at(&mut self, now: Duration) -> Result<(), Rejection> {
        let now = now.max(self.last_now);
        self.last_now = now;
        if let Some(until) = self.blocked {
            if now < until {
                return Err(Rejection::new(Reason::Cooldown, Some(until - now)));
            }
            self.reset();
        }
        if self
            .window
            .zip(self.started)
            .is_some_and(|(window, start)| now.saturating_sub(start) >= window)
        {
            self.failures = 0;
            self.started = None;
        }
        Ok(())
    }
    pub fn record_failure_at(&mut self, now: Duration) -> Result<(), Rejection> {
        self.check_at(now)?;
        let now = self.last_now;
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
