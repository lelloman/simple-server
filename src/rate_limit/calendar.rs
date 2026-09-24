use std::time::Duration;

/// Caller-owned counter keyed by an application-selected calendar period.
///
/// Period equality is the entire calendar rule: callers own time zone, date
/// formatting and persistence. A different period (even an earlier one) reads
/// as zero. Preflight never mutates stored state; recording reanchors it.
#[derive(Clone, Debug)]
pub struct CalendarCounter<D> {
    limit: u32,
    period: Option<D>,
    count: u32,
}
impl<D: Eq + Clone> CalendarCounter<D> {
    pub fn from_parts(limit: u32, period: Option<D>, count: u32) -> Self {
        Self {
            limit,
            period,
            count,
        }
    }
    pub fn period(&self) -> Option<&D> {
        self.period.as_ref()
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn usage_at(&self, period: &D) -> u32 {
        if self.period.as_ref() == Some(period) {
            self.count
        } else {
            0
        }
    }
    pub fn allows_at(&self, period: &D) -> bool {
        self.usage_at(period) < self.limit
    }
    /// Recording is independent of admission, including late or in-flight
    /// outcomes. Pathological counter overflow saturates instead of wrapping.
    pub fn record_at(&mut self, period: D) {
        self.count = self.usage_at(&period).saturating_add(1);
        self.period = Some(period);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarDenial {
    Interval { retry_after: Duration },
    Quota,
}

/// A persisted minimum gap followed by a calendar-period quota, in that order.
/// Check and record are separate; persistence and exclusive access stay local.
#[derive(Clone, Debug)]
pub struct CalendarGate<D> {
    minimum_interval: Duration,
    last: Option<Duration>,
    counter: CalendarCounter<D>,
}
impl<D: Eq + Clone> CalendarGate<D> {
    pub fn from_parts(
        minimum_interval: Duration,
        limit: u32,
        last: Option<Duration>,
        period: Option<D>,
        count: u32,
    ) -> Self {
        Self {
            minimum_interval,
            last,
            counter: CalendarCounter::from_parts(limit, period, count),
        }
    }
    pub fn last_admitted(&self) -> Option<Duration> {
        self.last
    }
    pub fn period(&self) -> Option<&D> {
        self.counter.period()
    }
    pub fn count(&self) -> u32 {
        self.counter.count()
    }
    pub fn check_at(&self, now: Duration, period: &D) -> Result<(), CalendarDenial> {
        if let Some(last) = self.last {
            let elapsed = now.saturating_sub(last);
            if elapsed < self.minimum_interval {
                return Err(CalendarDenial::Interval {
                    retry_after: self.minimum_interval - elapsed,
                });
            }
        }
        if !self.counter.allows_at(period) {
            return Err(CalendarDenial::Quota);
        }
        Ok(())
    }
    pub fn record_at(&mut self, now: Duration, period: D) {
        self.last = Some(now);
        self.counter.record_at(period);
    }
}
