//! HTTP serving over Unix domain sockets through the shared engine.
//!
//! Included in `engine-web` on Unix; the source backend's `unix-http` feature
//! is not needed. Requires an engine with Unix listener operations. Applications
//! own stale-file removal, permissions, signals, deadlines and final path cleanup.
//! No socket file is removed or replaced by this module. Peer credentials are
//! not provided here; RequestMetadata::peer is None.
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

/// Clones share the engine's HTTP/1 connection pool. Request and response bodies
/// stream without collection. Applications own deadlines and header policy.
#[derive(Clone)]
pub struct UnixClient {
    resource: Resource,
}
impl std::fmt::Debug for UnixClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnixClient").finish_non_exhaustive()
    }
}
impl UnixClient {
    /// Create an engine-owned pool. Older engines return an error here.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(14, json!({"op":"unix_client"}))?,
        })
    }
    /// Select the socket independently of the URI authority, preserving method,
    /// path/query, headers (including Host), version and streamed body/trailers.
    /// Socket paths must be nonempty UTF-8 and request paths origin-form, matching
    /// the source Unix client. Arbitrary Rust extensions and protocol upgrades
    /// do not cross the ABI. Run inside an engine runtime.
    pub async fn request(
        &self,
        socket_path: impl AsRef<Path>,
        request: super::Request,
    ) -> Result<super::Response, UnixHttpError> {
        let socket = socket_path
            .as_ref()
            .to_str()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| UnixHttpError::invalid("socket path must be nonempty UTF-8"))?;
        let path = request.uri().path_and_query().map_or("/", |v| v.as_str());
        if !path.starts_with('/') {
            return Err(UnixHttpError::invalid("request path must be origin-form"));
        }
        use std::fmt::Write;
        let mut encoded = String::with_capacity(socket.len() * 2);
        for byte in socket.as_bytes() {
            write!(encoded, "{byte:02x}").expect("formatting into a string");
        }
        let uri: http::Uri = format!("unix://{encoded}:0{path}")
            .parse()
            .map_err(|error| UnixHttpError {
                kind: UnixHttpErrorKind::InvalidRequest,
                source: Box::new(error),
            })?;
        self.send(uri, request)
            .await
            .map_err(UnixHttpError::transport)
    }
    async fn send(&self, uri: http::Uri, request: super::Request) -> io::Result<super::Response> {
        use super::HttpBody;
        let (parts, body) = request.into_parts();
        let hint = body.size_hint();
        let callback = super::body::export(body)?;
        // Preallocate the response registration, so cancellation cannot leave
        // an unclaimed body ID after async headers arrive.
        let resource = wire::resource(6, json!({"op":"server_body_slot"}))?;
        let output = Operation::new(&wire::encode(json!({"op":"unix_request","client":self.resource.id(),"response":resource.id(),"uri":uri.to_string(),"method":parts.method.as_str(),"version":format!("{:?}",parts.version),"headers":wire::headers(&parts.headers),"body":callback.id(),"lower":hint.lower(),"upper":hint.upper()}), &[])?)?.await?;
        let (header, _) = wire::decode(&output)?;
        wire::check(&header)?;
        let status = header["status"]
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .and_then(|v| http::StatusCode::from_u16(v).ok())
            .ok_or_else(|| io::Error::other("invalid response status"))?;
        let headers = wire::read_headers(&header["headers"])?;
        let version = wire::version(&header)?;
        let mut response = super::Response::new(super::Body::incoming(resource, &header)?);
        *response.status_mut() = status;
        *response.headers_mut() = headers;
        *response.version_mut() = version;
        Ok(response)
    }
}
/// Distinguish invalid local inputs from native connection/HTTP failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnixHttpErrorKind {
    InvalidRequest,
    Transport,
}
/// Transport sources are host-owned diagnostics, not native Rust error objects.
#[derive(Debug)]
pub struct UnixHttpError {
    kind: UnixHttpErrorKind,
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl UnixHttpError {
    fn invalid(message: &'static str) -> Self {
        Self {
            kind: UnixHttpErrorKind::InvalidRequest,
            source: Box::new(io::Error::new(io::ErrorKind::InvalidInput, message)),
        }
    }
    fn transport(error: io::Error) -> Self {
        Self {
            kind: UnixHttpErrorKind::Transport,
            source: Box::new(error),
        }
    }
    pub fn kind(&self) -> UnixHttpErrorKind {
        self.kind
    }
}
impl std::fmt::Display for UnixHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unix HTTP request failed ({:?})", self.kind)
    }
}
impl std::error::Error for UnixHttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}
