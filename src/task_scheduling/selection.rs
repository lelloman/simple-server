//! Selection policies over caller-owned queues and transactional claim backends.
use super::ConfigError;
use std::{
    future::Future,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

/// Weighted cyclic selection. Clones share the fairness cursor, not job ownership.
/// A backend callback must atomically decide eligibility and acquire its durable
/// claim. Empty attempts and errors do not consume weight. Concurrent callers may
/// inspect the same lane; the backend remains the authority for exclusivity.
#[derive(Clone, Debug)]
pub struct WeightedSelection<T> {
    lanes: Arc<[T]>,
    cursor: Arc<AtomicUsize>,
}
impl<T: Clone> WeightedSelection<T> {
    pub fn new(weights: impl IntoIterator<Item = (T, NonZeroUsize)>) -> Result<Self, ConfigError> {
        let mut lanes = Vec::new();
        for (lane, weight) in weights {
            lanes
                .try_reserve(weight.get())
                .map_err(|_| ConfigError("weighted cycle is too large".into()))?;
            lanes.extend(std::iter::repeat_n(lane, weight.get()));
        }
        if lanes.is_empty() {
            return Err(ConfigError("weighted selection needs a lane".into()));
        }
        Ok(Self {
            lanes: lanes.into(),
            cursor: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Probe at most one full cycle, preserving the specified slot order. Commit
    /// cursor advancement only after the backend returns a successful claim.
    /// Cancellation cannot undo a backend transaction that already committed.
    pub async fn claim_next<C, E, F, Fut>(&self, mut claim: F) -> Result<Option<C>, E>
    where
        F: FnMut(T) -> Fut,
        Fut: Future<Output = Result<Option<C>, E>>,
    {
        let start = self.cursor.load(Ordering::Relaxed) % self.lanes.len();
        for offset in 0..self.lanes.len() {
            let index = (start + offset) % self.lanes.len();
            if let Some(claim) = claim(self.lanes[index].clone()).await? {
                self.cursor
                    .store((index + 1) % self.lanes.len(), Ordering::Relaxed);
                return Ok(Some(claim));
            }
        }
        Ok(None)
    }
}

/// A batch's fullness can trigger immediately. Underfilled batches must meet the
/// minimum and the selected age threshold. Model/queue identity, cancellation
/// pruning and runner reservation remain caller-owned.
#[derive(Clone, Copy, Debug)]
pub struct BatchReadiness {
    pub minimum: usize,
    pub fill_timeout: Duration,
    pub saturation_timeout: Duration,
}
impl BatchReadiness {
    pub fn ready(
        &self,
        pending: usize,
        target: usize,
        age: Option<Duration>,
        saturated: bool,
    ) -> bool {
        pending != 0
            && (pending >= target
                || (pending >= self.minimum
                    && age.is_some_and(|age| {
                        age >= if saturated {
                            self.saturation_timeout
                        } else {
                            self.fill_timeout
                        }
                    })))
    }
}

/// An application-defined resource dimension sampled under the caller's durable
/// transaction. Zero limits disable admission. There is no in-memory semaphore
/// or implicit reservation: persist acceptance before releasing that transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceDemand {
    pub used: u128,
    pub requested: u128,
    pub limit: u128,
}
impl ResourceDemand {
    pub fn fits(self) -> bool {
        self.used
            .checked_add(self.requested)
            .is_some_and(|total| total <= self.limit)
    }
}

/// Report every failed dimension in input order, for deterministic admission
/// decisions and diagnostics. Overflow rejects rather than wrapping capacity.
pub fn denied_resources<'a>(
    demands: impl IntoIterator<Item = (&'a str, ResourceDemand)>,
) -> Vec<&'a str> {
    demands
        .into_iter()
        .filter_map(|(name, demand)| (!demand.fits()).then_some(name))
        .collect()
}

/// Caller-defined preference order. Unknown values sort after configured ones;
/// duplicate entries retain their last position, like an indexed map. Business
/// tie breakers remain explicit in the caller's composite sort key.
#[derive(Clone, Debug)]
pub struct PreferenceOrder<T> {
    positions: std::collections::HashMap<T, usize>,
}
impl<T: Eq + std::hash::Hash> PreferenceOrder<T> {
    pub fn new(values: impl IntoIterator<Item = T>) -> Self {
        Self {
            positions: values
                .into_iter()
                .enumerate()
                .map(|(i, v)| (v, i))
                .collect(),
        }
    }
    pub fn position<Q: ?Sized + Eq + std::hash::Hash>(&self, value: &Q) -> usize
    where
        T: std::borrow::Borrow<Q>,
    {
        self.positions.get(value).copied().unwrap_or(usize::MAX)
    }
}

/// Key for optional ranks: every known value precedes missing values, including
/// the largest representable rank (no sentinel collision).
pub fn missing_last<T>(value: Option<T>) -> (bool, Option<T>) {
    (value.is_none(), value)
}
