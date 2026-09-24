use super::{Admission, Budget, Quota, Reason, Rejection};
use std::{
    collections::HashMap,
    hash::Hash,
    num::{NonZeroU32, NonZeroUsize},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Monotonic elapsed time from a stable origin. Custom clocks enable deterministic
/// tests. Clones of a limiter share one clock and one store.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Duration;
}
pub struct SystemClock(Instant);
impl Default for SystemClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Overflow {
    /// Reject unseen keys at capacity without disturbing existing budgets.
    #[default]
    Reject,
    /// All unseen keys share one additional bucket until room is available.
    /// It is retained even if the main map later has room.
    SharedBucket,
}
#[derive(Clone, Copy, Debug)]
pub struct StoreConfig {
    capacity: Option<NonZeroUsize>,
    idle: Option<Duration>,
    overflow: Overflow,
}
impl StoreConfig {
    pub fn bounded(capacity: NonZeroUsize, idle: Duration) -> Self {
        Self {
            capacity: Some(capacity),
            idle: Some(idle),
            overflow: Overflow::Reject,
        }
    }
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }
    /// Explicit compatibility escape hatch. No capacity cap or automatic expiry.
    /// Prefer bounded stores for new deployments; this is never the default.
    pub fn unbounded() -> Self {
        Self {
            capacity: None,
            idle: None,
            overflow: Overflow::Reject,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoreStats {
    pub entries: usize,
    pub evictions: u64,
    pub overflow_checks: u64,
    pub active: usize,
}
struct Entry {
    budget: Budget,
    last_seen: Duration,
    active: usize,
}
impl Entry {
    fn new(quota: Quota, now: Duration) -> Self {
        Self {
            budget: Budget::new(quota),
            last_seen: now,
            active: 0,
        }
    }
    fn removable(&self, now: Duration, idle: Duration) -> bool {
        self.active == 0
            && now.saturating_sub(self.last_seen) >= idle
            && self.budget.is_replenished_at(now)
    }
}
struct State<K> {
    entries: HashMap<K, Entry>,
    overflow: Entry,
    evictions: u64,
    overflow_checks: u64,
    last_now: Duration,
}

/// Atomic process-local keyed budgets. Global budgets use `K = ()`; composite
/// keys can include route/user/device/endpoint class. No identity parsing occurs.
/// The map's capacity and optional overflow bucket do not bound key byte length:
/// applications should use bounded/hashed keys for untrusted input.
pub struct KeyedLimiter<K> {
    state: Arc<Mutex<State<K>>>,
    clock: Arc<dyn Clock>,
    quota: Quota,
    config: StoreConfig,
}
impl<K> Clone for KeyedLimiter<K> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            clock: self.clock.clone(),
            quota: self.quota,
            config: self.config,
        }
    }
}
impl<K: Clone + Eq + Hash + Send + Sync + 'static> KeyedLimiter<K> {
    pub fn new(quota: Quota, config: StoreConfig) -> Self {
        Self::with_clock(quota, config, SystemClock::default())
    }
    pub fn with_clock(quota: Quota, config: StoreConfig, clock: impl Clock) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                entries: HashMap::new(),
                overflow: Entry::new(quota, Duration::ZERO),
                evictions: 0,
                overflow_checks: 0,
                last_now: Duration::ZERO,
            })),
            clock: Arc::new(clock),
            quota,
            config,
        }
    }
    /// Charge rate only. For concurrent-work admission use `acquire` and hold its
    /// guard for the whole operation (or let the HTTP layer hold the body guard).
    pub fn check(&self, key: K, cost: NonZeroU32) -> Result<(), Rejection> {
        self.acquire(key, cost, None).map(drop)
    }
    /// Check concurrency BEFORE charging rate, under the same lock. Failed rate
    /// admission does not occupy a slot. Dropping the guard releases a slot only.
    pub fn acquire(
        &self,
        key: K,
        cost: NonZeroU32,
        concurrency: Option<NonZeroUsize>,
    ) -> Result<Admission, Rejection> {
        if cost.get() > self.quota.capacity().get() {
            return Err(Rejection::new(Reason::CostExceedsCapacity, None));
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = self.clock.now().max(state.last_now);
        state.last_now = now;
        let full = |state: &State<K>| {
            self.config
                .capacity
                .is_some_and(|cap| state.entries.len() >= cap.get())
        };
        if !state.entries.contains_key(&key) && full(&state) {
            Self::prune_state(&mut state, now, self.config.idle.unwrap_or_default());
        }
        let overflow = !state.entries.contains_key(&key) && full(&state);
        if overflow && matches!(self.config.overflow, Overflow::Reject) {
            return Err(Rejection::new(Reason::StoreCapacity, None));
        }
        let entry = if overflow {
            state.overflow_checks = state.overflow_checks.saturating_add(1);
            &mut state.overflow
        } else {
            state
                .entries
                .entry(key.clone())
                .or_insert_with(|| Entry::new(self.quota, now))
        };
        entry.last_seen = now;
        if concurrency.is_some_and(|max| entry.active >= max.get()) {
            return Err(Rejection::new(Reason::Concurrency, None));
        }
        entry.budget.check_at(now, cost)?;
        if concurrency.is_some() {
            entry.active += 1;
            Ok(Admission::holding(Lease {
                state: self.state.clone(),
                key: if overflow { None } else { Some(key) },
            }))
        } else {
            Ok(Admission::unrestricted())
        }
    }
    /// Explicit maintenance; no background worker is installed. Only idle, fully
    /// replenished entries without live permits are removed. Returns removed count.
    pub fn prune(&self) -> usize {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = self.clock.now().max(state.last_now);
        state.last_now = now;
        Self::prune_state(&mut state, now, self.config.idle.unwrap_or_default())
    }
    fn prune_state(state: &mut State<K>, now: Duration, idle: Duration) -> usize {
        let before = state.entries.len();
        state.entries.retain(|_, entry| !entry.removable(now, idle));
        let removed = before - state.entries.len();
        state.evictions = state.evictions.saturating_add(removed as u64);
        removed
    }
    pub fn stats(&self) -> StoreStats {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        StoreStats {
            entries: state.entries.len(),
            evictions: state.evictions,
            overflow_checks: state.overflow_checks,
            active: state.entries.values().map(|e| e.active).sum::<usize>() + state.overflow.active,
        }
    }
}
struct Lease<K: Eq + Hash> {
    state: Arc<Mutex<State<K>>>,
    key: Option<K>,
}
impl<K: Eq + Hash> Drop for Lease<K> {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let entry = match &self.key {
            Some(key) => state
                .entries
                .get_mut(key)
                .expect("live permits prevent eviction"),
            None => &mut state.overflow,
        };
        entry.active -= 1;
    }
}
