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

#[cfg(any(feature = "client", feature = "process", feature = "sqlite-client"))]
mod engine_wire;

/// Outbound HTTP executed by the engine, without a downstream HTTP client stack.
#[cfg(feature = "client")]
pub mod client;

/// Asynchronous child processes executed by the engine.
#[cfg(feature = "process")]
pub mod process;

/// Timers owned by the engine runtime.
#[cfg(feature = "runtime")]
pub mod time {
    pub use simple_server_sys::{Elapsed, advance, sleep, timeout};
}

/// Optional, driver-independent database contracts.
#[cfg(any(
    feature = "database-sqlite",
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

#[cfg(feature = "lifecycle")]
pub mod lifecycle;

/// Explicit process-wide logging setup, independent of HTTP and lifecycle.
#[cfg(feature = "logging")]
pub mod logging;

/// Optional request identifiers, independent of logging initialization.
#[cfg(feature = "correlation")]
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

/// Bounded scheduling with application-owned execution and reporting.
#[cfg(feature = "task-scheduling")]
pub mod task_scheduling;

/// Optional execution budgets, retries, circuit breakers and pause state.
#[cfg(feature = "task-policies")]
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
