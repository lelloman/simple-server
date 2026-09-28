//! TCP binding and private HTTP transport adapter.
//!
//! Public serving is available through `web::serve` and
//! `web::serve_with_connect_info` with owned router contracts.

#[cfg(feature = "web")]
use std::convert::Infallible;
use std::io;

#[cfg(feature = "web")]
use axum::{extract::Request, response::Response, serve::IncomingStream};
use tokio::net::{TcpListener, ToSocketAddrs};
#[cfg(feature = "web")]
use tower_service::Service;

#[cfg(feature = "web")]
use crate::lifecycle::Shutdown;

/// Bind a TCP listener before starting services. Port zero is supported, and
/// the actual address is available through `listener.local_addr()`.
pub async fn bind(address: impl ToSocketAddrs) -> io::Result<TcpListener> {
    TcpListener::bind(address).await
}

/// Serve the internal backend router or make-service on an already-bound listener.
///
/// Accepts `Router::into_make_service_with_connect_info::<SocketAddr>()` as well
/// as an ordinary router. This future installs no signals and chooses no timeout.
/// Dropping it does not guarantee termination of Axum's spawned connection tasks.
#[cfg(feature = "web")]
pub(crate) async fn serve<M, S>(
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
