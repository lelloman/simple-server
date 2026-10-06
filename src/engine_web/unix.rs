//! HTTP serving over Unix domain sockets through the shared engine.
//!
//! Included in `engine-web` on Unix; the source backend's `unix-http` feature
//! is not needed. Requires an engine with Unix listener operations. Applications
//! own stale-file removal, permissions, signals, deadlines and final path cleanup.
//! No socket file is removed or replaced by this module. Unix HTTP clients and
//! peer credentials are not provided here; RequestMetadata::peer is None.
use super::{Router, Shutdown, serve_resource, wire};
use serde_json::json;
use simple_server_sys::{Operation, Resource};
use std::{
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

/// A bound native Unix listener. Dropping it closes the socket but leaves the
/// filesystem entry in place. Serving consumes it; it cannot be cloned.
pub struct UnixListener {
    resource: Resource,
    path: PathBuf,
}
impl UnixListener {
    /// The path supplied to bind, preserving its original bytes.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Bind a filesystem socket without removing an existing path or changing its
/// permissions. Non-UTF-8 paths are preserved. Run inside an engine runtime.
pub async fn bind(path: impl AsRef<Path>) -> io::Result<UnixListener> {
    let path = path.as_ref().to_owned();
    let output = Operation::new(&wire::encode(
        json!({"op":"server_unix_bind"}),
        path.as_os_str().as_bytes(),
    )?)?
    .await?;
    let (header, _) = wire::decode(&output)?;
    wire::check(&header)?;
    Ok(UnixListener {
        resource: Resource::new(13, wire::id(&header)?),
        path,
    })
}

/// Stop accepting when shutdown is requested, then drain active responses.
/// Pre-requested shutdown accepts no requests. Apply an application deadline
/// separately; dropping this future alone does not stop all spawned connections.
/// The path remains on disk after shutdown, for the application to clean up.
pub async fn serve(listener: UnixListener, router: Router, shutdown: Shutdown) -> io::Result<()> {
    serve_resource(listener.resource, router, shutdown, "server_unix_serve").await
}
