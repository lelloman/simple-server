//! Optional rate budgets, durable quota policies, outcome counters and admission.
//!
//! Keys, proxy trust, route scope, identity and durable transactions belong to the
//! application. Base policy checks never sleep or spawn work. The opt-in
//! `rate-limit-async` adapter explicitly waits and schedules permit release.
//! Rate charges are not refunded; concurrency guards are released on drop.
//! The HTTP adapter preserves streaming.
mod budget;
mod flow;
mod layer;
mod outcomes;
mod per_second_bucket;
mod rolling;
mod store;
mod token_bucket;

pub use budget::{
    BlockedFailures, Budget, ConfigError, CooldownExpiry, FailureCounter, FailurePolicy, Quota,
    Reason, RefillPolicy, Rejection,
};
pub use flow::{Admission, AdmissionFuture, AsyncPolicy, Policy};
pub use layer::{RateLimitBody, RateLimitLayer, RateLimitService, rejection_response};
pub use outcomes::{FailureLatch, FailureWindow};
pub use per_second_bucket::PerSecondTokenBucket;
pub use store::{Clock, KeyedLimiter, Overflow, StoreConfig, StoreStats, SystemClock};

pub use token_bucket::{ClockRegression, TokenBucket};

pub use rolling::{
    RollingBudget, RollingCutoffError, RollingWindow, RollingWindowError, RollingWindowStore,
    RollingWindowUsage, evaluate_rolling_windows,
};

mod calendar;
mod limits;
mod window_counters;
pub use calendar::{CalendarCounter, CalendarDenial, CalendarGate};
pub use limits::{LimitCheck, evaluate_limits};
pub use window_counters::{WindowBoundary, WindowCounters, WindowDenial};

mod polling;
pub use polling::{PollingDenial, PollingGate};
#[cfg(feature = "rate-limit-async")]
mod delayed_release;
#[cfg(feature = "rate-limit-async")]
pub use delayed_release::DelayedReleaseLimiter;
