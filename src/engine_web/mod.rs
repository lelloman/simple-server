//! HTTP routing and streaming through the shared engine, without Axum or Tokio
//! in the consumer graph. Handlers accept an owned `Request` and return `Response`.
//!
//! This is an explicit development API, not yet a drop-in replacement for `web`:
//! TLS and protocol upgrades remain unimplemented. Tower layers wrap the opaque
//! `Route` service with infallible responses and streaming byte bodies. Bind application state with `with_state`.
//! Register handlers and serve inside the application's engine runtime.
//! Typed handlers allow body extraction only in the final position:
//! ```compile_fail
//! use simple_server::engine_web::{MethodRouter, Method, Json, State};
//! async fn invalid(_: Json<String>, _: State<()>) {}
//! let _ = MethodRouter::new().unwrap().on_handler(Method::POST, invalid);
//! ```
//! Two body consumers cannot be combined:
//! ```compile_fail
//! use simple_server::engine_web::{MethodRouter, Method, Json};
//! async fn invalid(_: Json<String>, _: String) {}
//! let _ = MethodRouter::new().unwrap().on_handler(Method::POST, invalid);
//! ```
pub mod body;
mod continuation;
pub use continuation::Route;
pub mod extract;
#[allow(clippy::duplicate_mod)]
#[path = "../web/handler.rs"]
mod handler;
pub mod middleware;
pub mod multipart;
mod path_de;
pub mod response;
mod routing;
mod service;
pub mod sse;
#[cfg(unix)]
pub mod unix;
pub use crate::extract::{Extract, FromRequestParts, IntoRejectionResponse, RejectionResponse};
pub use extract::{
    BodyLimit, ConnectInfo, Extension, Form, FromRequest, FromState, Json, MatchedPath, Path,
    Query, RawQuery, State,
};
pub use handler::Handler;
pub use response::IntoResponse;
/// Runtime-independent Tower service contract for engine request adapters.
pub use tower_service::Service;
mod wire;
pub use crate::engine_lifecycle::Shutdown;
pub use body::{Body, BodyError};
pub use bytes::Bytes;
pub use http::{self, HeaderMap, HeaderValue, Method, StatusCode, Uri, Version, header};
pub use http_body::{Body as HttpBody, Frame, SizeHint};
pub use routing::{MethodRouter, Router};
use serde_json::json;
use simple_server_sys::{Callback, Operation, Resource};
use std::{io, net::SocketAddr};

pub type Request = http::Request<Body>;
pub type Response = http::Response<Body>;

/// Engine routing metadata attached to request extensions. Headers preserve
/// binary values and duplicates; the direct peer is not a forwarded address.
#[derive(Clone, Debug)]
pub struct RequestMetadata {
    pub peer: Option<SocketAddr>,
    pub matched_path: Option<String>,
    pub original_uri: Option<Uri>,
    pub path_params: Vec<(String, String)>,
    /// Decoding failures remain available to raw handlers without silently
    /// pretending an invalid path parameter was absent.
    pub path_error: Option<(StatusCode, String)>,
}

/// An owned engine TCP listener. Dropping it before serving releases its socket.
/// It is intentionally non-cloneable: serving consumes the listener.
pub struct TcpListener {
    resource: Resource,
    address: SocketAddr,
}
impl TcpListener {
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
}
pub async fn bind(address: impl AsRef<str>) -> io::Result<TcpListener> {
    let bytes = Operation::new(&wire::encode(
        json!({"op":"server_bind","address":address.as_ref()}),
        &[],
    )?)?
    .await?;
    let (header, _) = wire::decode(&bytes)?;
    wire::check(&header)?;
    let resource = Resource::new(7, wire::id(&header)?);
    let address = header["address"]
        .as_str()
        .ok_or_else(|| io::Error::other("missing bound address"))?
        .parse()
        .map_err(io::Error::other)?;
    Ok(TcpListener { resource, address })
}

/// Stop accepting when requested, then drain active responses. Apply an
/// application deadline separately. Dropping this future alone does not promise
/// cancellation of spawned connections; use explicit shutdown for graceful drain.
pub async fn serve(listener: TcpListener, router: Router, shutdown: Shutdown) -> io::Result<()> {
    serve_resource(listener.resource, router, shutdown, "server_serve").await
}

async fn serve_resource(
    listener: Resource,
    router: Router,
    shutdown: Shutdown,
    operation: &'static str,
) -> io::Result<()> {
    let router = router.into_resource()?;
    let callback = Callback::new(move |_| {
        let shutdown = shutdown.clone();
        async move {
            shutdown.requested().await;
            Vec::new()
        }
    })?;
    let operation = Operation::new(&wire::encode(
        json!({"op":operation, "listener":listener.id(), "router":router.id(), "shutdown":callback.id()}),
        &[],
    )?)?;
    let output = operation.await?;
    wire::check(&wire::decode(&output)?.0)
}
