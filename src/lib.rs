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

#[cfg(all(feature = "lifecycle", feature = "http"))]
pub mod http;
