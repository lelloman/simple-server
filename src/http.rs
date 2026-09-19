//! TCP/HTTP lifecycle adapters. Requires both `http` and `lifecycle` features.
//!
//! Axum types remain part of this transitional API. A shutdown request initiates
//! graceful HTTP draining; use the lifecycle coordinator to bound the wait.
//! Upgraded connections and application-spawned tasks require their own tracking.

use std::{convert::Infallible, io};

use axum::{extract::Request, response::Response, serve::IncomingStream};
use tokio::net::{TcpListener, ToSocketAddrs};
use tower_service::Service;

use crate::lifecycle::Shutdown;

/// Bind a TCP listener before starting services. Port zero is supported, and
/// the actual address is available through `listener.local_addr()`.
pub async fn bind(address: impl ToSocketAddrs) -> io::Result<TcpListener> {
    TcpListener::bind(address).await
}

/// Serve an Axum router or make-service on an already-bound listener.
///
/// Accepts `Router::into_make_service_with_connect_info::<SocketAddr>()` as well
/// as an ordinary router. This future installs no signals and chooses no timeout.
/// Dropping it does not guarantee termination of Axum's spawned connection tasks.
pub async fn serve<M, S>(
    listener: TcpListener,
    make_service: M,
    shutdown: Shutdown,
) -> io::Result<()>
where
    M: for<'a> Service<IncomingStream<'a, TcpListener>, Error = Infallible, Response = S>
        + Send
        + 'static,
    for<'a> <M as Service<IncomingStream<'a, TcpListener>>>::Future: Send,
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    // Do not start accepting when shutdown was requested before polling us.
    if shutdown.is_requested() {
        return Ok(());
    }
    axum::serve(listener, make_service)
        .with_graceful_shutdown(async move { shutdown.requested().await })
        .await
}
