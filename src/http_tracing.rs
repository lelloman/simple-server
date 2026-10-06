//! Private backend engine for the public `web::tracing` contract. It records
//! safe route templates, status, header latency and body lifetime without
//! buffering or reading bodies.
//! It does not install a subscriber. See `docs/step-03c-http-tracing.md` for
//! event semantics, placement and upgrade boundaries.

use axum::{
    body::{Body, Bytes, HttpBody},
    extract::MatchedPath,
    http::{Method, Request, StatusCode},
    response::Response,
};
include!("http_tracing_core.rs");
