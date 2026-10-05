//! TLS transport for owned [`super::Router`] routes.
//!
//! Certificate policy and generation belong to the application. This adapter
//! accepts PEM material, serves an already-bound listener, and observes the
//! same sticky shutdown signal as the plain HTTP server.

use std::{io, path::Path};

use crate::net::TcpListener;
use axum_server::tls_rustls::RustlsConfig;

use super::Router;
use crate::lifecycle::Shutdown;

/// A TLS certificate and private key loaded from PEM material.
#[derive(Clone)]
pub struct TlsConfig(RustlsConfig);

impl TlsConfig {
    /// Parse a PEM certificate chain and private key from memory.
    pub async fn from_pem(cert: Vec<u8>, key: Vec<u8>) -> io::Result<Self> {
        RustlsConfig::from_pem(cert, key).await.map(Self)
    }

    /// Read and parse a PEM certificate chain and private key from files.
    pub async fn from_pem_file(cert: impl AsRef<Path>, key: impl AsRef<Path>) -> io::Result<Self> {
        RustlsConfig::from_pem_file(cert, key).await.map(Self)
    }
}

/// Serve HTTPS and WebSocket upgrades until shutdown is requested.
///
/// Shutdown stops accepting connections and waits for active connections to
/// finish. The lifecycle coordinator may bound that wait. Applications remain
/// responsible for closing their upgraded sockets and background tasks.
pub async fn serve(
    listener: TcpListener,
    router: Router,
    config: TlsConfig,
    shutdown: Shutdown,
) -> io::Result<()> {
    if shutdown.is_requested() {
        return Ok(());
    }

    let handle = axum_server::Handle::new();
    let server = axum_server::from_tcp_rustls(listener.into_std()?, config.0)
        .handle(handle.clone())
        .serve(router.inner.into_make_service());
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => result,
        _ = shutdown.requested() => {
            handle.graceful_shutdown(None);
            server.await
        }
    }
}
