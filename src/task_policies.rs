//! Storage-independent execution policies. All state-machine clocks and jitter
//! samples are caller supplied. Snapshots contain control state, never job claims.
use std::{
    collections::BTreeSet,
    fmt,
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyError(pub &'static str);
impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for PolicyError {}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutionBudget {
    pub queue_timeout: Option<Duration>,
    pub max_runtime: Option<Duration>,
}
impl ExecutionBudget {
    pub fn validate(&self) -> Result<(), PolicyError> {
        for duration in [self.queue_timeout, self.max_runtime].into_iter().flatten() {
            if duration.is_zero() || Instant::now().checked_add(duration).is_none() {
                return Err(PolicyError(
                    "budgets must be nonzero representable durations",
                ));
            }
        }
        Ok(())
    }
    pub fn queue_deadline(&self, eligible: Instant) -> Option<Instant> {
        eligible.checked_add(self.queue_timeout?)
    }
    pub fn runtime_deadline(&self, started: Instant) -> Option<Instant> {
        started.checked_add(self.max_runtime?)
    }
}
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_attempts: NonZeroU32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub jitter: Duration,
}
impl RetryPolicy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.initial_delay.is_zero()
            || self.max_delay < self.initial_delay
            || Instant::now().checked_add(self.max_delay).is_none()
        {
            return Err(PolicyError(
                "retry delays must be nonzero, ordered and representable",
            ));
        }
        Ok(())
    }
    /// Attempt one is the initial execution. The cap includes positive jitter.
    pub fn delay_after(&self, attempt: u32, sample: f64) -> Option<Duration> {
        if attempt == 0 || attempt >= self.max_attempts.get() || self.validate().is_err() {
            return None;
        }
        let cap = self.max_delay.as_nanos();
        let nanos = 1u128
            .checked_shl(attempt - 1)
            .and_then(|factor| self.initial_delay.as_nanos().checked_mul(factor))
            .unwrap_or(cap)
            .min(cap);
        let delay = Duration::new(
            (nanos / 1_000_000_000) as u64,
            (nanos % 1_000_000_000) as u32,
        );
        Some(
            delay
                .saturating_add(sample_jitter(self.jitter, sample))
                .min(self.max_delay),
        )
    }
}
pub(crate) fn sample_jitter(maximum: Duration, sample: f64) -> Duration {
    let sample = if sample.is_finite() {
        sample.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let nanos = ((maximum.as_nanos() as f64 * sample) as u128).min(maximum.as_nanos());
    Duration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    )
}
#[derive(Clone, Debug)]
pub struct CircuitPolicy {
    pub failure_threshold: NonZeroU32,
    pub cooldown: Duration,
}
impl CircuitPolicy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.cooldown.is_zero() || SystemTime::now().checked_add(self.cooldown).is_none() {
            Err(PolicyError(
                "circuit cooldown must be nonzero and representable",
            ))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CircuitSnapshot {
    pub consecutive_failures: u32,
    pub open_until: Option<SystemTime>,
}
#[derive(Debug)]
pub struct CircuitPermit {
    generation: u64,
    probe: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CircuitOutcome {
    Success,
    Failure,
    Ignored,
}
#[derive(Clone, Debug)]
pub struct CircuitBreaker {
    policy: CircuitPolicy,
    state: CircuitSnapshot,
    generation: u64,
    probe_active: bool,
}
impl CircuitBreaker {
    pub fn new(policy: CircuitPolicy) -> Result<Self, PolicyError> {
        policy.validate()?;
        Ok(Self {
            policy,
            state: CircuitSnapshot::default(),
            generation: 0,
            probe_active: false,
        })
    }
    pub fn restore(policy: CircuitPolicy, state: CircuitSnapshot) -> Result<Self, PolicyError> {
        let mut breaker = Self::new(policy)?;
        if state.open_until.is_none()
            && state.consecutive_failures >= breaker.policy.failure_threshold.get()
        {
            return Err(PolicyError(
                "closed circuit snapshot exceeds failure threshold",
            ));
        }
        breaker.state = state;
        Ok(breaker)
    }
    pub fn snapshot(&self) -> CircuitSnapshot {
        self.state.clone()
    }
    pub fn can_admit(&self, now: SystemTime) -> bool {
        !self.probe_active && self.state.open_until.is_none_or(|until| until <= now)
    }
    pub fn admit(&mut self, now: SystemTime) -> Option<CircuitPermit> {
        if !self.can_admit(now) {
            return None;
        }
        let probe = self.state.open_until.is_some();
        if probe {
            self.generation = self.generation.wrapping_add(1);
            self.probe_active = true;
        }
        Some(CircuitPermit {
            generation: self.generation,
            probe,
        })
    }
    /// Outcomes from an earlier open/half-open generation cannot alter current state.
    pub fn record(&mut self, permit: CircuitPermit, outcome: CircuitOutcome, now: SystemTime) {
        if permit.generation != self.generation {
            return;
        }
        if permit.probe {
            self.probe_active = false;
        }
        match outcome {
            CircuitOutcome::Ignored => {}
            CircuitOutcome::Success => {
                self.state = CircuitSnapshot::default();
                if permit.probe {
                    self.generation = self.generation.wrapping_add(1);
                }
            }
            CircuitOutcome::Failure => {
                self.state.consecutive_failures = self.state.consecutive_failures.saturating_add(1);
                if permit.probe
                    || self.state.consecutive_failures >= self.policy.failure_threshold.get()
                {
                    self.state.open_until = now.checked_add(self.policy.cooldown).or(Some(now));
                    self.generation = self.generation.wrapping_add(1);
                }
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PauseScope {
    Global,
    Pool(String),
    Job(String),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PauseState {
    pub global: bool,
    pub pools: BTreeSet<String>,
    pub jobs: BTreeSet<String>,
}
impl PauseState {
    pub fn is_paused(&self, job: &str, pool: Option<&str>) -> bool {
        self.global || self.jobs.contains(job) || pool.is_some_and(|pool| self.pools.contains(pool))
    }
    pub fn set(&mut self, scope: &PauseScope, paused: bool) {
        match scope {
            PauseScope::Global => self.global = paused,
            PauseScope::Pool(pool) => {
                if paused {
                    self.pools.insert(pool.clone());
                } else {
                    self.pools.remove(pool);
                }
            }
            PauseScope::Job(job) => {
                if paused {
                    self.jobs.insert(job.clone());
                } else {
                    self.jobs.remove(job);
                }
            }
        }
    }
}
type Classifier<E> = Arc<dyn Fn(&E) -> bool + Send + Sync>;
pub struct ExecutionPolicy<E> {
    pub budget: ExecutionBudget,
    pub circuit: Option<CircuitPolicy>,
    retry: Option<(RetryPolicy, Classifier<E>)>,
}
impl<E> Default for ExecutionPolicy<E> {
    fn default() -> Self {
        Self {
            budget: ExecutionBudget::default(),
            circuit: None,
            retry: None,
        }
    }
}
impl<E> ExecutionPolicy<E> {
    pub fn with_retry(
        mut self,
        policy: RetryPolicy,
        classify: impl Fn(&E) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.retry = Some((policy, Arc::new(classify)));
        self
    }
    pub fn validate(&self) -> Result<(), PolicyError> {
        self.budget.validate()?;
        if let Some(circuit) = &self.circuit {
            circuit.validate()?;
        }
        if let Some((policy, _)) = &self.retry {
            policy.validate()?;
        }
        Ok(())
    }
    pub fn retry_delay(&self, error: &E, attempt: u32, sample: f64) -> Option<Duration> {
        let (policy, classify) = self.retry.as_ref()?;
        if classify(error) {
            policy.delay_after(attempt, sample)
        } else {
            None
        }
    }
}
