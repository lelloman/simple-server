//! Common response conversions without a source HTTP framework.
use super::{Body, Form, HeaderMap, StatusCode};
pub use super::{Json, Response};
use crate::extract::RejectionResponse;
pub trait IntoResponse {
    fn into_response(self) -> Response;
}
impl<B> IntoResponse for http::Response<B>
where
    Body: From<B>,
{
    fn into_response(self) -> Response {
        self.map(Body::from)
    }
}
impl IntoResponse for RejectionResponse {
    fn into_response(self) -> Response {
        self.into_http().map(Body::from)
    }
}
impl IntoResponse for Body {
    fn into_response(self) -> Response {
        Response::new(self)
    }
}
impl IntoResponse for () {
    fn into_response(self) -> Response {
        Body::empty().into_response()
    }
}
impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        (self, ()).into_response()
    }
}
fn content(body: Body, kind: &'static str) -> Response {
    let mut response = Response::new(body);
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static(kind),
    );
    response
}
macro_rules! primitive {
    ($kind:literal; $($ty:ty),*) => {$(impl IntoResponse for $ty { fn into_response(self)->Response { content(Body::from(self),$kind) } })*};
}
primitive!("text/plain; charset=utf-8"; String, &'static str);
primitive!("application/octet-stream"; super::Bytes, Vec<u8>, &'static [u8]);
impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(bytes) => content(Body::from(bytes), "application/json"),
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        }
    }
}
impl<T: serde::Serialize> IntoResponse for Form<T> {
    fn into_response(self) -> Response {
        match serde_urlencoded::to_string(&self.0) {
            Ok(body) => content(Body::from(body), "application/x-www-form-urlencoded"),
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        }
    }
}
impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(value) => value.into_response(),
            Err(error) => error.into_response(),
        }
    }
}
impl<R: IntoResponse> IntoResponse for (StatusCode, R) {
    fn into_response(self) -> Response {
        let mut response = self.1.into_response();
        *response.status_mut() = self.0;
        response
    }
}
impl<R: IntoResponse> IntoResponse for (HeaderMap, R) {
    fn into_response(self) -> Response {
        let mut response = self.1.into_response();
        response.headers_mut().extend(self.0);
        response
    }
}
impl<R: IntoResponse> IntoResponse for (StatusCode, HeaderMap, R) {
    fn into_response(self) -> Response {
        (self.0, (self.1, self.2)).into_response()
    }
}
impl IntoResponse for HeaderMap {
    fn into_response(self) -> Response {
        (self, ()).into_response()
    }
}
impl IntoResponse for std::convert::Infallible {
    fn into_response(self) -> Response {
        match self {}
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Html<T>(pub T);
impl<T> IntoResponse for Html<T>
where
    Body: From<T>,
{
    fn into_response(self) -> Response {
        content(Body::from(self.0), "text/html; charset=utf-8")
    }
}

// Convert the body first, then replace headers in order. A failed conversion
// discards the whole original response, including already-applied headers.
fn array_response<K, V, const N: usize>(
    headers: [(K, V); N],
    mut response: Response,
    status: Option<StatusCode>,
) -> Response
where
    K: TryInto<http::header::HeaderName>,
    K::Error: std::fmt::Display,
    V: TryInto<http::HeaderValue>,
    V::Error: std::fmt::Display,
{
    for (name, value) in headers {
        let name = match name.try_into() {
            Ok(name) => name,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        };
        let value = match value.try_into() {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        };
        response.headers_mut().insert(name, value);
    }
    if let Some(status) = status {
        *response.status_mut() = status;
    }
    response
}
/// Arrays replace header values; use HeaderMap::append for repeated fields.
impl<K, V, R, const N: usize> IntoResponse for ([(K, V); N], R)
where
    K: TryInto<http::header::HeaderName>,
    K::Error: std::fmt::Display,
    V: TryInto<http::HeaderValue>,
    V::Error: std::fmt::Display,
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        array_response(self.0, self.1.into_response(), None)
    }
}
impl<K, V, R, const N: usize> IntoResponse for (StatusCode, [(K, V); N], R)
where
    K: TryInto<http::header::HeaderName>,
    K::Error: std::fmt::Display,
    V: TryInto<http::HeaderValue>,
    V::Error: std::fmt::Display,
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        array_response(self.1, self.2.into_response(), Some(self.0))
    }
}
impl<K, V, const N: usize> IntoResponse for [(K, V); N]
where
    K: TryInto<http::header::HeaderName>,
    K::Error: std::fmt::Display,
    V: TryInto<http::HeaderValue>,
    V::Error: std::fmt::Display,
{
    fn into_response(self) -> Response {
        (self, ()).into_response()
    }
}

/// An HTTP redirect. Invalid Location header values become a plain-text 500
/// during response conversion, matching the source backend.
#[derive(Clone, Debug)]
pub struct Redirect {
    status: StatusCode,
    location: String,
}
impl Redirect {
    /// Redirect with 303 See Other.
    pub fn to(uri: &str) -> Self {
        Self {
            status: StatusCode::SEE_OTHER,
            location: uri.to_owned(),
        }
    }
    /// Redirect with 307 Temporary Redirect, preserving the request method.
    pub fn temporary(uri: &str) -> Self {
        Self {
            status: StatusCode::TEMPORARY_REDIRECT,
            location: uri.to_owned(),
        }
    }
    /// Redirect with 308 Permanent Redirect, preserving the request method.
    pub fn permanent(uri: &str) -> Self {
        Self {
            status: StatusCode::PERMANENT_REDIRECT,
            location: uri.to_owned(),
        }
    }
    pub fn status_code(&self) -> StatusCode {
        self.status
    }
    pub fn location(&self) -> &str {
        &self.location
    }
}
impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        array_response(
            [(http::header::LOCATION, self.location)],
            ().into_response(),
            Some(self.status),
        )
    }
}
