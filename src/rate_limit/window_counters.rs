use std::time::Duration;

/// Whether a retained anchor expires at the exact window boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowBoundary {
    AtOrAfter,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowDenial {
    /// Exact remaining duration. A strict boundary can deny with zero remaining;
    /// the application selects rounding and a minimum for its wire response.
    pub retry_after: Duration,
}

/// Caller-owned category counters with a shared, explicitly initialized anchor.
///
/// The caller owns its clock origin, keys and synchronization. Reads never reset
/// state. Refresh/check reset every category together and reanchor to the sample;
/// missed windows are not replayed. Backward samples observe zero elapsed.
#[derive(Clone, Debug)]
pub struct WindowCounters<const N: usize> {
    window: Duration,
    boundary: WindowBoundary,
    anchor: Duration,
    counts: [u64; N],
}

impl<const N: usize> WindowCounters<N> {
    pub fn new(window: Duration, boundary: WindowBoundary, anchor: Duration) -> Self {
        Self {
            window,
            boundary,
            anchor,
            counts: [0; N],
        }
    }

    pub fn anchor(&self) -> Duration {
        self.anchor
    }
    pub fn counts(&self) -> &[u64; N] {
        &self.counts
    }

    pub fn refresh_at(&mut self, now: Duration) -> bool {
        let elapsed = now.saturating_sub(self.anchor);
        let expired = match self.boundary {
            WindowBoundary::AtOrAfter => elapsed >= self.window,
            WindowBoundary::After => elapsed > self.window,
        };
        if expired {
            self.anchor = now;
            self.counts.fill(0);
        }
        expired
    }

    /// Non-consuming preflight, including maintenance on idle loop ticks.
    /// Zero limits reject positive costs. Category indexes must be below N.
    pub fn check_at(
        &mut self,
        now: Duration,
        index: usize,
        limit: u64,
        cost: u64,
    ) -> Result<(), WindowDenial> {
        self.refresh_at(now);
        if self.counts[index]
            .checked_add(cost)
            .is_some_and(|next| next <= limit)
        {
            Ok(())
        } else {
            Err(WindowDenial {
                retry_after: self.window.saturating_sub(now.saturating_sub(self.anchor)),
            })
        }
    }

    /// Charge only after successful preflight. Denials never charge any category.
    pub fn admit_at(
        &mut self,
        now: Duration,
        index: usize,
        limit: u64,
        cost: u64,
    ) -> Result<(), WindowDenial> {
        self.check_at(now, index, limit, cost)?;
        self.record(index, cost);
        Ok(())
    }

    /// Record an attempted outcome separately from preflight, without refreshing
    /// the window or rechecking a limit. Counters saturate rather than wrap.
    pub fn record(&mut self, index: usize, cost: u64) {
        self.counts[index] = self.counts[index].saturating_add(cost);
    }
}
