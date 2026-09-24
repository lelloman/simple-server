use std::fmt;

/// A read-only quota over events strictly newer than a rolling cutoff.
///
/// Durations and limits may be zero. Timestamps use signed microseconds in the
/// caller's time domain (normally UTC), allowing windows to cross the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RollingWindow {
    pub window_micros: u64,
    pub limit: u64,
}

/// Application-owned time and durable event storage.
///
/// Implementations must count only the events belonging to this policy, strictly
/// after `cutoff_micros`. There is intentionally no upper time bound. The index
/// is the window's position in the supplied slice, allowing distinct scopes.
/// The application owns transactions, locking and successful-event recording;
/// evaluation does not reserve or consume capacity. Callers requiring atomic
/// admission must provide it around evaluation and subsequent accounting.
pub trait RollingWindowStore {
    type Error;

    /// Sample once per window, preserving stores that read each window in turn.
    fn now_micros(&mut self) -> Result<i128, Self::Error>;
    fn count_after(&mut self, window_index: usize, cutoff_micros: i128)
    -> Result<u64, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollingWindowUsage {
    pub cutoff_micros: i128,
    pub used: u64,
    pub remaining: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollingBudget {
    pub remaining: u64,
    /// Results in configuration order, including windows after an exhausted one.
    pub windows: Vec<RollingWindowUsage>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RollingWindowError<E> {
    Backend(E),
    TimestampRange { window_index: usize },
}

impl<E: fmt::Display> fmt::Display for RollingWindowError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "rolling-window backend: {error}"),
            Self::TimestampRange { window_index } => {
                write!(
                    f,
                    "rolling-window cutoff out of range at index {window_index}"
                )
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RollingWindowError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::TimestampRange { .. } => None,
        }
    }
}

/// Evaluate durable sliding quotas without changing their event history.
///
/// Computes each strict cutoff, asks the store for usage, and takes the minimum
/// saturating remaining count. `unlimited_budget` is used only when no windows
/// are configured. All windows are evaluated even after exhaustion so usage
/// details remain complete and later storage errors cannot become success.
/// No retry, cache, fallback on errors, clock clamping or charge is introduced.
pub fn evaluate_rolling_windows<S: RollingWindowStore>(
    windows: &[RollingWindow],
    unlimited_budget: u64,
    store: &mut S,
) -> Result<RollingBudget, RollingWindowError<S::Error>> {
    let mut result = RollingBudget {
        remaining: u64::MAX,
        windows: Vec::with_capacity(windows.len()),
    };
    for (window_index, window) in windows.iter().enumerate() {
        let now = store.now_micros().map_err(RollingWindowError::Backend)?;
        let cutoff_micros = now
            .checked_sub(i128::from(window.window_micros))
            .ok_or(RollingWindowError::TimestampRange { window_index })?;
        let used = store
            .count_after(window_index, cutoff_micros)
            .map_err(RollingWindowError::Backend)?;
        let remaining = window.limit.saturating_sub(used);
        result.remaining = result.remaining.min(remaining);
        result.windows.push(RollingWindowUsage {
            cutoff_micros,
            used,
            remaining,
        });
    }
    if windows.is_empty() {
        result.remaining = unlimited_budget;
    }
    Ok(result)
}
