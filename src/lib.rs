//! A modular foundation for Axum-based Rust services.
//!
//! The first migration stage centralizes the Axum dependency. Consumers use
//! `simple_server::axum` while retaining their current routers, state,
//! middleware, startup, and shutdown behavior. The re-export is transitional;
//! focused modules will eventually hide Axum behind this crate's own interfaces.
//!
//! The default `http` feature exposes Axum with its standard default features.
//! `ws`, `multipart`, `macros`, and `http2` opt into additional Axum capabilities.
//! Disable default features to use this crate without an HTTP dependency.
//!
//! The opt-in `lifecycle` feature adds shutdown notification, explicit signal
//! registration, and coordination of application-owned service futures. Combined
//! with `http`, it also provides TCP binding and graceful HTTP serving adapters.

/// The centrally versioned HTTP framework, exposed during incremental migration.
#[cfg(feature = "http")]
pub use axum;

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
pub mod http_tracing;

#[cfg(all(feature = "lifecycle", feature = "http"))]
pub mod http;

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
