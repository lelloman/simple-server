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

/// The centrally versioned HTTP framework, exposed during incremental migration.
#[cfg(feature = "http")]
pub use axum;
