//! Optional, process-local rate budgets, outcome counters and admission policies.
//!
//! Keys, proxy trust, route scope, identity and durable transactions belong to the
//! application. Checks never sleep or spawn work. Rate charges are not refunded;
//! concurrency guards are released on drop. The HTTP adapter preserves streaming.
mod budget;
mod flow;
mod layer;
mod store;
mod token_bucket;

pub use budget::{Budget, ConfigError, FailureCounter, Quota, Reason, Rejection};
pub use flow::{Admission, AdmissionFuture, AsyncPolicy, Policy};
pub use layer::{RateLimitBody, RateLimitLayer, RateLimitService, rejection_response};
pub use store::{Clock, KeyedLimiter, Overflow, StoreConfig, StoreStats, SystemClock};

pub use token_bucket::TokenBucket;
