//! Modular server infrastructure with owned HTTP interfaces. Axum is an
//! implementation detail of the optional HTTP capabilities.
//!
//! The default `http` feature enables the HTTP backend; `web` provides the
//! public routing, handler, extractor, response and serving contracts. Disable
//! default features to use this crate without an HTTP dependency.
//!
//! The opt-in `lifecycle` feature adds shutdown notification, explicit signal
//! registration, and coordination of application-owned service futures. Combined
//! with `http`, it also provides TCP binding. `web` owns HTTP serving.

#[cfg(all(test, feature = "runtime"))]
extern crate self as simple_server;

/// Execution through the separately built engine runtime.
#[cfg(feature = "runtime")]
pub mod runtime {
    pub use simple_server_sys::{
        AbortHandle, Builder, Handle, Id, JoinError, JoinHandle, JoinSet, Runtime, spawn,
        spawn_blocking, yield_now,
    };
}

#[cfg(feature = "runtime")]
pub use simple_server_macros::{main, test};

#[cfg(any(
    feature = "client",
    feature = "process",
    feature = "sqlite-client",
    feature = "postgres-client",
    feature = "oidc"
))]
mod engine_wire;

/// OpenID Connect authorization code flows executed by the engine.
#[cfg(feature = "oidc")]
pub mod oidc;

/// Outbound HTTP executed by the engine, without a downstream HTTP client stack.
#[cfg(feature = "client")]
pub mod client;

/// Asynchronous child processes executed by the engine.
#[cfg(feature = "process")]
pub mod process;

/// Timers owned by the engine runtime.
#[cfg(feature = "runtime")]
pub mod time {
    /// Policy for a periodic timer whose scheduled tick is more than 5ms late.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum MissedTickBehavior {
        /// Catch up using the original scheduled ticks.
        #[default]
        Burst,
        /// Schedule the next tick one period after the late tick is observed.
        Delay,
        /// Skip missed ticks while preserving the original interval alignment.
        Skip,
    }
    /// Fixed-period timer; the first tick is immediate. Default policy is Burst.
    pub struct Interval {
        next: Instant,
        period: std::time::Duration,
        behavior: MissedTickBehavior,
    }
    pub fn interval(period: std::time::Duration) -> Interval {
        assert!(!period.is_zero());
        Interval {
            next: Instant::now(),
            period,
            behavior: MissedTickBehavior::Burst,
        }
    }
    impl Interval {
        pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
            self.behavior = behavior;
        }
        pub async fn tick(&mut self) -> Instant {
            let now = self.next;
            sleep_until(now).await;
            let observed = Instant::now();
            self.next = if observed > now + std::time::Duration::from_millis(5) {
                match self.behavior {
                    MissedTickBehavior::Burst => now + self.period,
                    MissedTickBehavior::Delay => observed + self.period,
                    MissedTickBehavior::Skip => {
                        let remainder = (observed - now).as_nanos() % self.period.as_nanos();
                        let remainder = std::time::Duration::new(
                            (remainder / 1_000_000_000)
                                .try_into()
                                .expect("duration seconds"),
                            (remainder % 1_000_000_000) as u32,
                        );
                        observed + (self.period - remainder)
                    }
                }
            } else {
                now + self.period
            };
            now
        }
    }

    pub use simple_server_sys::{
        Elapsed, Instant, Sleep, advance, sleep, sleep_until, timeout, timeout_at,
    };
}

#[cfg(feature = "engine-web")]
pub mod engine_web;

/// Optional, driver-independent database contracts.
#[cfg(any(
    feature = "database-sqlite",
    feature = "postgres-client",
    feature = "database-migrations",
    feature = "database-blocking"
))]
pub mod database;

/// Framework-independent HTTP routing, handlers, extractors and responses.
#[cfg(feature = "web")]
pub mod web;

/// Application-owned request extraction with an internal HTTP framework adapter.
#[cfg(feature = "extract")]
pub mod extract;

#[cfg(any(
    feature = "lifecycle",
    feature = "engine-lifecycle",
    feature = "engine-tasks"
))]
mod shutdown;

#[cfg(feature = "lifecycle")]
pub mod lifecycle;

/// Lifecycle coordination and explicit signals executed by the shared engine.
#[cfg(feature = "engine-lifecycle")]
pub mod engine_lifecycle;

/// Explicit process-wide logging setup, independent of HTTP and lifecycle.
#[cfg(feature = "logging")]
pub mod logging;

/// Optional request identifiers, independent of logging initialization.
#[cfg(feature = "correlation-core")]
pub mod correlation;

/// Optional HTTP request spans and response-body lifecycle events.
#[cfg(feature = "http-tracing")]
mod http_tracing;

#[cfg(all(feature = "lifecycle", feature = "http"))]
pub mod http;

/// Owned listener and asynchronous address-resolution contracts.
#[cfg(all(feature = "lifecycle", feature = "http"))]
pub mod net;

/// Explicit route-scoped extractor body limits.
#[cfg(feature = "body-limit")]
pub mod body_limit;

/// Explicit response-header policies using framework-independent HTTP primitives.
#[cfg(feature = "response-headers")]
pub mod response_headers;

/// Explicit, framework-independent cross-origin response policy.
#[cfg(feature = "cors")]
pub mod cors;

/// Application-owned liveness and readiness checks with optional HTTP adaptation.
#[cfg(feature = "health")]
pub mod health;

/// Explicit task ownership and cooperative cancellation.
#[cfg(feature = "tasks")]
pub mod tasks;

#[cfg(feature = "engine-tasks")]
pub mod engine_tasks;

/// Bounded scheduling with application-owned execution and reporting.
#[cfg(feature = "task-scheduling")]
#[path = "task_scheduling/source.rs"]
pub mod task_scheduling;

#[cfg(feature = "engine-scheduling")]
pub mod engine_scheduling;

/// Engine-backed bounded batches and durable-work polling without the full scheduler.
#[cfg(feature = "task-drivers")]
pub mod task_drivers;

/// Optional execution budgets, retries, circuit breakers and pause state.
#[cfg(feature = "task-policy-core")]
pub mod task_policies;

/// Combined authentication and authorization without Axum or a required runtime.
#[cfg(feature = "auth")]
pub mod auth;

/// Optional rate budgets, outcome counters and HTTP admission without Axum.
#[cfg(feature = "rate-limit")]
pub mod rate_limit;

/// Optional HTTP fixtures using owned routers, requests and responses.
#[cfg(feature = "test-harness")]
pub mod testing;

#[cfg(feature = "engine-io")]
pub mod fs;
#[cfg(feature = "engine-io")]
pub mod io {
    pub use futures_util::io::*;
}
#[cfg(feature = "engine-io")]
#[path = "watch.rs"]
pub mod watch;
#[cfg(feature = "engine-io")]
pub mod sync {
    pub use crate::watch;
    pub use async_lock::{Mutex, MutexGuardArc, OnceCell, RwLock, Semaphore, SemaphoreGuardArc};
    pub use futures_channel::oneshot;
}

/// Synchronous Zstd byte-buffer codecs implemented in the native engine.
#[cfg(feature = "zstd")]
pub mod zstd;

/// Native filtering and formatting behind a thin host tracing subscriber.
#[cfg(feature = "engine-logging")]
pub mod engine_logging;
#[cfg(any(feature = "logging", feature = "engine-logging"))]
mod logging_options;

#[cfg(feature = "hashing")]
pub mod hashing;

#[cfg(all(feature = "runtime", unix))]
pub mod signal {
    pub use simple_server_sys::{UnixSignal, UnixSignalKind};
}
