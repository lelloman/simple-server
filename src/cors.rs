//! Explicit CORS policy; nothing is enabled just by selecting the feature.
//!
//! [`CorsConfig::default`](crate::cors::CorsConfig::default) grants no cross-origin permissions. Applications own
//! origins, credentials and layer placement. CORS controls browser access to
//! responses; it is not authentication or CSRF protection. All OPTIONS requests
//! are answered by the layer without calling the inner service.
//!
//! ```
//! use simple_server::cors::CorsConfig;
//! use http::{HeaderValue, Method, header};
//! let layer = CorsConfig::default()
//!     .allow_origins([HeaderValue::from_static("https://app.example")])
//!     .allow_methods([Method::GET, Method::POST])
//!     .allow_headers([header::CONTENT_TYPE])
//!     .allow_credentials(true)
//!     .build()?;
//! # Ok::<(), simple_server::cors::CorsError>(())
//! ```

use http::{HeaderName, HeaderValue, Method, Request, Response};
use std::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tower_layer::Layer;
use tower_service::Service;

#[derive(Clone, Debug)]
enum Selection<T> {
    List(Vec<T>),
    Any,
}
impl<T> Default for Selection<T> {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

/// A policy builder with no origins, methods, allowed or exposed headers by default.
/// Setters replace previous values; empty lists revoke that selection.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct CorsConfig {
    origins: Selection<HeaderValue>,
    methods: Selection<Method>,
    headers: Selection<HeaderName>,
    exposed: Selection<HeaderName>,
    credentials: bool,
    max_age: Option<Duration>,
}

impl CorsConfig {
    /// Set the exact origin values. Literal `*` entries are rejected by `build`.
    pub fn allow_origins(mut self, values: impl IntoIterator<Item = HeaderValue>) -> Self {
        self.origins = Selection::List(values.into_iter().collect());
        self
    }
    /// Explicitly allow any origin; incompatible with credentials.
    pub fn allow_any_origin(mut self) -> Self {
        self.origins = Selection::Any;
        self
    }
    /// Set the HTTP methods. Literal `*` entries are rejected by `build`.
    pub fn allow_methods(mut self, values: impl IntoIterator<Item = Method>) -> Self {
        self.methods = Selection::List(values.into_iter().collect());
        self
    }
    /// Explicitly allow any HTTP method; incompatible with credentials.
    pub fn allow_any_method(mut self) -> Self {
        self.methods = Selection::Any;
        self
    }
    /// Set the request headers. Literal `*` entries are rejected by `build`.
    pub fn allow_headers(mut self, values: impl IntoIterator<Item = HeaderName>) -> Self {
        self.headers = Selection::List(values.into_iter().collect());
        self
    }
    /// Explicitly allow any request header; incompatible with credentials.
    pub fn allow_any_header(mut self) -> Self {
        self.headers = Selection::Any;
        self
    }
    /// Set the response headers exposed to scripts. Literal `*` entries are rejected by `build`.
    pub fn expose_headers(mut self, values: impl IntoIterator<Item = HeaderName>) -> Self {
        self.exposed = Selection::List(values.into_iter().collect());
        self
    }
    /// Explicitly expose any response header to scripts; incompatible with credentials.
    pub fn expose_any_header(mut self) -> Self {
        self.exposed = Selection::Any;
        self
    }
    /// Whether browsers may use credentials. Defaults to false.
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.credentials = allow;
        self
    }
    /// Set preflight cache lifetime, serialized in whole seconds.
    pub fn max_age(mut self, age: Duration) -> Self {
        self.max_age = Some(age);
        self
    }
    /// Validate the policy before installing it. No Axum or implementation types
    /// appear in this API. Wildcards must use the explicit `any` setters.
    pub fn build(self) -> Result<CorsLayer, CorsError> {
        fn validate<T: AsRef<[u8]>>(
            s: &Selection<T>,
            field: &'static str,
            credentials: bool,
        ) -> Result<(), CorsError> {
            match s {
                Selection::Any if credentials => Err(CorsError {
                    field,
                    credentials: true,
                }),
                Selection::List(items) if items.iter().any(|v| v.as_ref() == b"*") => {
                    Err(CorsError {
                        field,
                        credentials: false,
                    })
                }
                _ => Ok(()),
            }
        }
        validate(&self.origins, "origins", self.credentials)?;
        // Method implements AsRef<str>, unlike HeaderName/HeaderValue.
        let methods = match &self.methods {
            Selection::Any => Selection::Any,
            Selection::List(items) => {
                Selection::List(items.iter().map(|v| v.as_str().as_bytes()).collect())
            }
        };
        validate(&methods, "methods", self.credentials)?;
        validate(&self.headers, "allowed headers", self.credentials)?;
        validate(&self.exposed, "exposed headers", self.credentials)?;
        use tower_http::cors::{AllowOrigin, Any};
        let mut inner = tower_http::cors::CorsLayer::new().allow_credentials(self.credentials);
        inner = match self.origins {
            Selection::List(v) => inner.allow_origin(AllowOrigin::list(v)),
            Selection::Any => inner.allow_origin(Any),
        };
        inner = match self.methods {
            Selection::List(v) => inner.allow_methods(v),
            Selection::Any => inner.allow_methods(Any),
        };
        inner = match self.headers {
            Selection::List(v) => inner.allow_headers(v),
            Selection::Any => inner.allow_headers(Any),
        };
        inner = match self.exposed {
            Selection::List(v) => inner.expose_headers(v),
            Selection::Any => inner.expose_headers(Any),
        };
        if let Some(age) = self.max_age {
            inner = inner.max_age(age);
        }
        Ok(CorsLayer { inner })
    }
}

/// Invalid wildcard selection. Returned at construction, not on the first request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorsError {
    field: &'static str,
    credentials: bool,
}
impl fmt::Display for CorsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.credentials {
            write!(
                f,
                "CORS {} cannot be wildcard when credentials are enabled",
                self.field
            )
        } else {
            write!(
                f,
                "CORS {} list contains '*'; use the explicit any setter",
                self.field
            )
        }
    }
}
impl std::error::Error for CorsError {}

/// Validated Tower layer. Cloning retains the same policy.
#[derive(Clone, Debug)]
pub struct CorsLayer {
    inner: tower_http::cors::CorsLayer,
}
impl<S> Layer<S> for CorsLayer {
    type Service = CorsService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CorsService {
            inner: self.inner.layer(inner),
        }
    }
}

/// A service applying a validated CORS policy without buffering response bodies.
#[derive(Clone, Debug)]
pub struct CorsService<S> {
    inner: tower_http::cors::Cors<S>,
}
impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CorsService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    ResBody: Default,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        ResponseFuture {
            inner: self.inner.call(request),
        }
    }
}
pin_project_lite::pin_project! {
    /// Response future preserving the inner service's error and body types.
    pub struct ResponseFuture<F> {
        #[pin]
        inner: tower_http::cors::ResponseFuture<F>,
    }
}
impl<F, B, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
    B: Default,
{
    type Output = Result<Response<B>, E>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}
