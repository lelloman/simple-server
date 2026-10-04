//! Mutable request cookies and response deltas, independent of the HTTP backend.
//!
//! Install [`CookieManagerLayer`] outside handlers and middleware that extract
//! [`Cookies`]. Clones share one request's mutations; the layer appends deltas
//! after the downstream response completes. Applications choose every cookie
//! attribute and own session, authentication and CSRF policy.
//!
//! ```
//! use simple_server::web::{Router, routing::get};
//! use simple_server::web::cookies::{Cookie, CookieManagerLayer, Cookies, SameSite};
//! async fn handler(cookies: Cookies) -> &'static str {
//!     cookies.add(Cookie::build(("preference", "compact"))
//!         .path("/").http_only(true).secure(true).same_site(SameSite::Lax).build());
//!     "saved"
//! }
//! let _: Router = Router::new().route("/", get(handler)).layer(CookieManagerLayer::new());
//! ```

use super::{FromRequestParts, RejectionResponse};
use http::{HeaderMap, HeaderValue, Request, Response, header, request::Parts};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
};
use tower_layer::Layer;
use tower_service::Service;

/// Standard cookie values, attributes and time types. These contain no HTTP
/// backend types; the mutable jar and middleware below belong to simple-server.
pub use cookie::{Cookie, CookieBuilder, Expiration, SameSite, time};

/// One request's shared, lazily parsed mutable cookie jar.
///
/// Repeated Cookie headers are read in order; the last cookie of a given name
/// wins. Invalid UTF-8 headers and cookies that cannot be decoded are skipped.
/// Reading alone never emits Set-Cookie. Clones share mutations, not snapshots.
#[derive(Clone, Debug, Default)]
pub struct Cookies {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    headers: Vec<HeaderValue>,
    jar: Option<cookie::CookieJar>,
}

impl Inner {
    fn jar(&mut self) -> &mut cookie::CookieJar {
        self.jar.get_or_insert_with(|| {
            let mut jar = cookie::CookieJar::new();
            for header in &self.headers {
                if let Ok(value) = std::str::from_utf8(header.as_bytes()) {
                    for value in value.split(';') {
                        if let Ok(cookie) = Cookie::parse_encoded(value.to_owned()) {
                            jar.add_original(cookie);
                        }
                    }
                }
            }
            jar
        })
    }
}

impl Cookies {
    /// Construct a detached jar from all incoming Cookie headers. This does not
    /// install response propagation; use the layer or explicitly append deltas.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                headers: headers.get_all(header::COOKIE).iter().cloned().collect(),
                jar: None,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A caller panic must not make subsequent extraction unusable. No user
        // callback runs under this lock; only cookie parsing/collection does.
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// Return an owned cookie value, releasing the lock before returning.
    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.lock().jar().get(name).cloned()
    }

    /// Add or replace a cookie by name. No attributes are added implicitly.
    pub fn add(&self, cookie: Cookie<'static>) {
        self.lock().jar().add(cookie);
    }

    /// Remove an incoming cookie, emitting its expiry delta. Supply its path
    /// and domain when needed. Adding then removing a new cookie emits nothing;
    /// removing a cookie absent from the original jar also emits nothing.
    pub fn remove(&self, cookie: Cookie<'static>) {
        self.lock().jar().remove(cookie);
    }

    /// Snapshot the current cookies. Ordering is unspecified.
    pub fn list(&self) -> Vec<Cookie<'static>> {
        self.lock().jar().iter().cloned().collect()
    }

    /// Snapshot additions/removals relative to incoming cookies. This neither
    /// clears nor drains them. Ordering is unspecified.
    pub fn delta(&self) -> Vec<Cookie<'static>> {
        let inner = self.lock();
        inner
            .jar
            .as_ref()
            .map_or_else(Vec::new, |jar| jar.delta().cloned().collect())
    }

    /// Append each representable delta as a separate Set-Cookie header,
    /// preserving existing headers. Returns the number appended. Values that
    /// cannot be represented as HTTP headers are skipped, matching the legacy
    /// cookie middleware. Inspect `delta()` for custom output validation.
    ///
    /// Deltas are not drained: call once per response to avoid duplicate output.
    pub fn append_delta(&self, headers: &mut HeaderMap) -> usize {
        let mut count = 0;
        for cookie in self.delta() {
            if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
                headers.append(header::SET_COOKIE, value);
                count += 1;
            }
        }
        count
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Cookies {
    type Rejection = RejectionResponse;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            let mut response = RejectionResponse::new(
                b"Can't extract cookies. Is `CookieManagerLayer` enabled?".to_vec(),
            );
            *response.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            response
        })
    }
}

/// Install a fresh request jar and append its response deltas. The layer does
/// not authenticate, sign/encrypt cookies, or choose attributes. Install once,
/// outside every middleware/handler that extracts the jar. Merely enabling the
/// Cargo feature does not install the layer.
#[derive(Clone, Copy, Debug, Default)]
pub struct CookieManagerLayer;

impl CookieManagerLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for CookieManagerLayer {
    type Service = CookieManager<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CookieManager { inner }
    }
}

/// Shared cookie middleware, generic over request/response bodies and errors.
#[derive(Clone, Debug)]
pub struct CookieManager<S> {
    inner: S,
}

impl<S, B, R> Service<Request<B>> for CookieManager<S>
where
    S: Service<Request<B>, Response = Response<R>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = CookieResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let cookies = Cookies::from_headers(request.headers());
        request.extensions_mut().insert(cookies.clone());
        CookieResponseFuture {
            future: self.inner.call(request),
            cookies,
        }
    }
}

pin_project_lite::pin_project! {
    /// Future used by the shared cookie middleware; no backend types are exposed.
    pub struct CookieResponseFuture<F> {
        #[pin]
        future: F,
        cookies: Cookies,
    }
}

impl<F, B, E> Future for CookieResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(Ok(mut response)) => {
                this.cookies.append_delta(response.headers_mut());
                Poll::Ready(Ok(response))
            }
            other => other,
        }
    }
}
