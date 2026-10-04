//! Optional streaming gzip response compression with owned HTTP boundaries.
//!
//! Installing the layer is explicit. The default policy matches the existing
//! gzip middleware: responses of at least 32 bytes (or unknown length), excluding
//! raster images, gRPC and SSE. SVG remains eligible; the gRPC prefix includes grpc-web. Encoded and
//! range responses are never recompressed, even with a custom predicate.
//!
//! ```
//! use simple_server::web::{Router, compression::CompressionLayer};
//! let app: Router = Router::new().layer(CompressionLayer::new());
//! ```

use super::{Body, Response};
use http::{Extensions, HeaderMap, Request, StatusCode, Version};
use std::{
    fmt,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower_http::compression::predicate::{NotForContentType, Predicate};
use tower_layer::Layer;
use tower_service::Service;

type ResponsePredicate = dyn Fn(StatusCode, Version, &HeaderMap, &Extensions) -> bool + Send + Sync;

#[derive(Clone)]
enum Policy {
    Default { min_size: u64 },
    Custom(Arc<ResponsePredicate>),
}
impl fmt::Debug for Policy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default { min_size } => f
                .debug_struct("Default")
                .field("min_size", min_size)
                .finish(),
            Self::Custom(_) => f.write_str("Custom(..)"),
        }
    }
}
impl Predicate for Policy {
    fn should_compress<B: http_body::Body>(&self, response: &http::Response<B>) -> bool {
        match self {
            Self::Default { min_size } => {
                let size = response.body().size_hint().exact().or_else(|| {
                    response
                        .headers()
                        .get(http::header::CONTENT_LENGTH)
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<u64>().ok())
                });
                size.is_none_or(|size| size >= *min_size)
                    && NotForContentType::const_new("application/grpc")
                        .and(NotForContentType::IMAGES)
                        .and(NotForContentType::SSE)
                        .should_compress(response)
            }
            Self::Custom(predicate) => predicate(
                response.status(),
                response.version(),
                response.headers(),
                response.extensions(),
            ),
        }
    }
}

/// Gzip quality choices independent of the compression backend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompressionQuality {
    #[default]
    Default,
    Fastest,
    Best,
}

/// Explicit gzip response policy. No compression is installed by a Cargo feature.
#[derive(Clone, Debug)]
#[must_use]
pub struct CompressionLayer {
    gzip: bool,
    quality: CompressionQuality,
    policy: Policy,
}
impl Default for CompressionLayer {
    fn default() -> Self {
        Self {
            gzip: true,
            quality: CompressionQuality::Default,
            policy: Policy::Default { min_size: 32 },
        }
    }
}
impl CompressionLayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable gzip; other encodings are not offered by this module.
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.gzip = enabled;
        self
    }

    pub fn quality(mut self, quality: CompressionQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Select a minimum known response size, retaining default MIME exclusions.
    /// Unknown-length streams remain eligible. Replaces a custom predicate.
    pub fn min_size(mut self, bytes: u64) -> Self {
        self.policy = Policy::Default { min_size: bytes };
        self
    }

    /// Replace the size/MIME policy using owned HTTP response metadata.
    /// Existing Content-Encoding and Content-Range still prevent compression.
    /// Called once before reading the response body; it cannot inspect payloads.
    pub fn compress_when(
        mut self,
        predicate: impl Fn(StatusCode, Version, &HeaderMap, &Extensions) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.policy = Policy::Custom(Arc::new(predicate));
        self
    }
}
impl<S> Layer<S> for CompressionLayer {
    type Service = CompressionService<S>;
    fn layer(&self, service: S) -> Self::Service {
        use tower_http::compression::CompressionLevel;
        let quality = match self.quality {
            CompressionQuality::Default => CompressionLevel::Default,
            CompressionQuality::Fastest => CompressionLevel::Fastest,
            CompressionQuality::Best => CompressionLevel::Best,
        };
        let inner = tower_http::compression::CompressionLayer::new()
            .gzip(self.gzip)
            // Pin the offered encodings even if another crate unifies features.
            .no_br()
            .no_deflate()
            .no_zstd()
            .quality(quality)
            .compress_when(self.policy.clone())
            .layer(service);
        CompressionService { inner }
    }
}

/// A streaming service retaining the inner service's readiness and errors.
#[derive(Clone)]
pub struct CompressionService<S> {
    inner: tower_http::compression::Compression<S, Policy>,
}
impl<S, ReqBody> Service<Request<ReqBody>> for CompressionService<S>
where
    S: Service<Request<ReqBody>, Response = Response>,
{
    type Response = Response;
    type Error = S::Error;
    type Future = CompressionResponseFuture<S::Future>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        CompressionResponseFuture {
            inner: self.inner.call(request),
        }
    }
}
pin_project_lite::pin_project! {
    /// Future returning the shared owned body, without collecting it.
    pub struct CompressionResponseFuture<F> {
        #[pin]
        inner: tower_http::compression::ResponseFuture<F, Policy>,
    }
}
impl<F, E> Future for CompressionResponseFuture<F>
where
    F: Future<Output = Result<Response, E>>,
{
    type Output = Result<Response, E>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project()
            .inner
            .poll(cx)
            .map(|result| result.map(|response| response.map(Body::new)))
    }
}
