//! Owned HTTP serving and streaming clients over Unix domain sockets.
//!
//! Applications bind/manage socket paths and permissions, install signals and
//! choose lifecycle grace periods. This module never removes socket files or
//! installs authentication, retries, request-ID/header policy or a timeout.
use super::{Body, Request, Response, Router};
use crate::lifecycle::Shutdown;
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector};
use std::{error::Error, fmt, io, path::Path};
use tokio::net::UnixListener;

/// Serve an already-bound listener with owned routes and explicit shutdown.
/// Pre-requested shutdown accepts no connections. Dropping this future alone
/// does not guarantee all spawned connections terminate; use lifecycle shutdown.
pub async fn serve(listener: UnixListener, router: Router, shutdown: Shutdown) -> io::Result<()> {
    if shutdown.is_requested() {
        return Ok(());
    }
    axum::serve(listener, router.inner)
        .with_graceful_shutdown(async move { shutdown.requested().await })
        .await
}

/// Clones share a connection pool. Request/response bodies remain streaming.
/// HTTP/1 is used by the client; applications own deadlines and header policy.
#[derive(Clone)]
pub struct UnixClient {
    inner: Client<UnixConnector, Body>,
}
impl fmt::Debug for UnixClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnixClient").finish_non_exhaustive()
    }
}
impl Default for UnixClient {
    fn default() -> Self {
        Self {
            inner: Client::unix(),
        }
    }
}
impl UnixClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the socket independently from the HTTP URI's authority.
    /// Retains method, path/query, headers, version, extensions and body. Host
    /// is not removed: proxy callers must apply their own forwarding policy.
    /// Requires a UTF-8 socket path and an origin-form path beginning with `/`;
    /// unsupported paths return an error rather than panicking or losing bytes.
    pub async fn request(
        &self,
        socket_path: impl AsRef<Path>,
        mut request: Request,
    ) -> Result<Response, UnixHttpError> {
        let socket = socket_path
            .as_ref()
            .to_str()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| UnixHttpError::invalid("socket path must be nonempty UTF-8"))?;
        let path = request.uri().path_and_query().map_or("/", |p| p.as_str());
        if !path.starts_with('/') {
            return Err(UnixHttpError::invalid("request path must be origin-form"));
        }
        use std::fmt::Write;
        let mut encoded = String::with_capacity(socket.len() * 2);
        for byte in socket.as_bytes() {
            write!(encoded, "{byte:02x}").expect("formatting into a string");
        }
        *request.uri_mut() = format!("unix://{encoded}:0{path}")
            .parse()
            .map_err(|error| UnixHttpError {
                kind: UnixHttpErrorKind::InvalidRequest,
                source: Box::new(error),
            })?;
        let response = self
            .inner
            .request(request)
            .await
            .map_err(|error| UnixHttpError {
                kind: UnixHttpErrorKind::Transport,
                source: Box::new(error),
            })?;
        Ok(response.map(Body::new))
    }
}

/// Distinguish invalid local request inputs from connection/HTTP transport errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnixHttpErrorKind {
    InvalidRequest,
    Transport,
}
#[derive(Debug)]
pub struct UnixHttpError {
    kind: UnixHttpErrorKind,
    source: Box<dyn Error + Send + Sync>,
}
impl UnixHttpError {
    fn invalid(message: &'static str) -> Self {
        Self {
            kind: UnixHttpErrorKind::InvalidRequest,
            source: Box::new(io::Error::new(io::ErrorKind::InvalidInput, message)),
        }
    }
    pub fn kind(&self) -> UnixHttpErrorKind {
        self.kind
    }
}
impl fmt::Display for UnixHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unix HTTP request failed ({:?})", self.kind)
    }
}
impl Error for UnixHttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
