/// Rejection from a persisted polling interval. The caller owns protocol mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollingDenial {
    pub retry_after_seconds: u128,
}

/// A caller-owned signed-epoch polling gate with explicit record/admit methods.
///
/// Raw signed elapsed time preserves backward-clock and zero/negative interval
/// semantics of existing stored configurations. Arithmetic widens to i128 to
/// avoid timestamp subtraction overflow. No interval escalation is implicit.
/// Persist the admitted timestamp in the application's existing transaction.
#[derive(Clone, Copy, Debug)]
pub struct PollingGate {
    interval_seconds: i64,
    last_poll_at: Option<i64>,
}
impl PollingGate {
    pub fn from_parts(interval_seconds: i64, last_poll_at: Option<i64>) -> Self {
        Self {
            interval_seconds,
            last_poll_at,
        }
    }
    pub fn last_poll_at(&self) -> Option<i64> {
        self.last_poll_at
    }
    pub fn check_at(&self, now_seconds: i64) -> Result<(), PollingDenial> {
        if let Some(last) = self.last_poll_at {
            let elapsed = i128::from(now_seconds) - i128::from(last);
            let remaining = i128::from(self.interval_seconds) - elapsed;
            if remaining > 0 {
                return Err(PollingDenial {
                    retry_after_seconds: remaining as u128,
                });
            }
        }
        Ok(())
    }
    pub fn record_at(&mut self, now_seconds: i64) {
        self.last_poll_at = Some(now_seconds);
    }
    pub fn admit_at(&mut self, now_seconds: i64) -> Result<(), PollingDenial> {
        self.check_at(now_seconds)?;
        self.record_at(now_seconds);
        Ok(())
    }
}
