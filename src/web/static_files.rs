//! Optional disk-backed static files with streaming bodies and owned HTTP types.
//!
//! Mount a directory with `Router::nest_service` or use it as a router fallback.
//! Missing files return 404 unless an explicit fallback file is configured.
//! GET/HEAD, ranges, modification-time conditionals, MIME detection and directory
//! redirects retain the backend's filesystem semantics. No caching policy or
//! dynamic response compression is installed.
//!
//! Paths containing parent segments or backslashes cannot escape the root.
//! Symlinks are followed: use an application-controlled root, not an untrusted
//! upload directory. An explicit fallback may serve its configured file for an
//! invalid path, but never serves the requested path outside the root.
//!
//! ```
//! use simple_server::web::{Router, static_files::StaticDir};
//! let app: Router = Router::new().fallback_service(
//!     StaticDir::new("frontend").fallback_file("frontend/index.html")
//! );
//! ```

use super::{Body, Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};
use tower_http::services::{ServeDir, ServeFile};
use tower_service::Service;

type ResponseFuture = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

/// A streaming file service. The request path does not select the file.
#[derive(Clone, Debug)]
pub struct StaticFile {
    inner: ServeFile,
    status: Option<http::StatusCode>,
}

impl StaticFile {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            inner: ServeFile::new(path),
            status: None,
        }
    }

    /// Allow a `.gz` sibling when the client accepts gzip; otherwise use the original.
    pub fn precompressed_gzip(mut self) -> Self {
        self.inner = self.inner.precompressed_gzip();
        self
    }

    /// Allow a `.br` sibling when the client accepts Brotli; otherwise use the original.
    pub fn precompressed_br(mut self) -> Self {
        self.inner = self.inner.precompressed_br();
        self
    }
}

impl Service<Request> for StaticFile {
    type Response = Response;
    type Error = Infallible;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        <ServeFile as Service<Request>>::poll_ready(&mut self.inner, cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let future = self.inner.call(request);
        let status = self.status;
        Box::pin(async move {
            let mut response = future.await?.map(Body::new);
            if let Some(status) = status {
                *response.status_mut() = status;
            }
            Ok(response)
        })
    }
}

#[derive(Clone, Debug)]
enum DirectoryBackend {
    Plain(ServeDir),
    Fallback(ServeDir<StaticFile>),
}

/// A directory service with optional, explicit SPA or error-page fallback.
/// Directory `index.html` lookup is enabled by default. Only GET/HEAD are served;
/// other methods return 405 without invoking the fallback.
#[derive(Clone, Debug)]
pub struct StaticDir {
    inner: DirectoryBackend,
}

impl StaticDir {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            inner: DirectoryBackend::Plain(ServeDir::new(path)),
        }
    }

    /// Enable/disable `index.html` lookup for directory requests (default: true).
    pub fn append_index_html_on_directories(mut self, enabled: bool) -> Self {
        self.inner = match self.inner {
            DirectoryBackend::Plain(dir) => {
                DirectoryBackend::Plain(dir.append_index_html_on_directories(enabled))
            }
            DirectoryBackend::Fallback(dir) => {
                DirectoryBackend::Fallback(dir.append_index_html_on_directories(enabled))
            }
        };
        self
    }

    /// Serve this file when a path is missing, preserving the file response status.
    /// Point to the frontend's `index.html` for a SPA fallback (normally 200).
    /// This also applies to missing asset paths; no extension heuristic is added.
    pub fn fallback_file(self, path: impl AsRef<Path>) -> Self {
        self.fallback(StaticFile::new(path))
    }

    /// Serve this file for missing paths with a 404 status, suitable for error pages.
    pub fn not_found_file(self, path: impl AsRef<Path>) -> Self {
        let mut file = StaticFile::new(path);
        file.status = Some(http::StatusCode::NOT_FOUND);
        self.fallback(file)
    }

    fn fallback(mut self, file: StaticFile) -> Self {
        self.inner = DirectoryBackend::Fallback(match self.inner {
            DirectoryBackend::Plain(dir) => dir.fallback(file),
            DirectoryBackend::Fallback(dir) => dir.fallback(file),
        });
        self
    }

    pub fn precompressed_gzip(mut self) -> Self {
        self.inner = match self.inner {
            DirectoryBackend::Plain(dir) => DirectoryBackend::Plain(dir.precompressed_gzip()),
            DirectoryBackend::Fallback(dir) => DirectoryBackend::Fallback(dir.precompressed_gzip()),
        };
        self
    }

    pub fn precompressed_br(mut self) -> Self {
        self.inner = match self.inner {
            DirectoryBackend::Plain(dir) => DirectoryBackend::Plain(dir.precompressed_br()),
            DirectoryBackend::Fallback(dir) => DirectoryBackend::Fallback(dir.precompressed_br()),
        };
        self
    }
}

impl Service<Request> for StaticDir {
    type Response = Response;
    type Error = Infallible;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        match &mut self.inner {
            DirectoryBackend::Plain(dir) => <ServeDir as Service<Request>>::poll_ready(dir, cx),
            DirectoryBackend::Fallback(dir) => {
                <ServeDir<StaticFile> as Service<Request>>::poll_ready(dir, cx)
            }
        }
    }

    fn call(&mut self, request: Request) -> Self::Future {
        match &mut self.inner {
            DirectoryBackend::Plain(dir) => {
                let future = dir.call(request);
                Box::pin(async move { Ok(future.await?.map(Body::new)) })
            }
            DirectoryBackend::Fallback(dir) => {
                let future = dir.call(request);
                Box::pin(async move { Ok(future.await?.map(Body::new)) })
            }
        }
    }
}
