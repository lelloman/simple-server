use super::Body;
pub use super::Json;
pub use super::Response;
use crate::extract::RejectionResponse;
use http::{HeaderMap, StatusCode};

/// Convert an application result into a shared HTTP response.
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

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::response::IntoResponse::into_response(axum::Json(self.0)).map(Body)
    }
}

macro_rules! primitive_response {
    ($($ty:ty),+ $(,)?) => {$(
        impl IntoResponse for $ty {
            fn into_response(self) -> Response {
                axum::response::IntoResponse::into_response(self).map(Body)
            }
        }
    )+};
}
primitive_response!(
    (),
    StatusCode,
    String,
    &'static str,
    Vec<u8>,
    bytes::Bytes,
    &'static [u8]
);

impl IntoResponse for Body {
    fn into_response(self) -> Response {
        Response::new(self)
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

impl IntoResponse for std::convert::Infallible {
    fn into_response(self) -> Response {
        match self {}
    }
}

/// Header arrays replace existing values, matching the HTTP tuple contract.
/// Use HeaderMap with append for repeated fields such as multiple cookies.
impl<K, V, R, const N: usize> IntoResponse for ([(K, V); N], R)
where
    K: TryInto<http::header::HeaderName>,
    K::Error: std::fmt::Display,
    V: TryInto<http::HeaderValue>,
    V::Error: std::fmt::Display,
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let response = self.1.into_response().map(|body| body.0);
        axum::response::IntoResponse::into_response((self.0, response)).map(Body)
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
        let response = self.2.into_response().map(|body| body.0);
        axum::response::IntoResponse::into_response((self.0, self.1, response)).map(Body)
    }
}

/// HTML bytes with the HTML UTF-8 content type.
#[derive(Debug, Clone, Copy)]
pub struct Html<T>(pub T);
impl<T> IntoResponse for Html<T>
where
    Body: From<T>,
{
    fn into_response(self) -> Response {
        (
            [("content-type", "text/html; charset=utf-8")],
            Body::from(self.0),
        )
            .into_response()
    }
}

/// An HTTP redirect with a validated Location response header.
#[derive(Debug, Clone)]
pub struct Redirect(axum::response::Redirect);
impl Redirect {
    pub fn to(uri: &str) -> Self {
        Self(axum::response::Redirect::to(uri))
    }
    pub fn temporary(uri: &str) -> Self {
        Self(axum::response::Redirect::temporary(uri))
    }
    pub fn permanent(uri: &str) -> Self {
        Self(axum::response::Redirect::permanent(uri))
    }
    pub fn status_code(&self) -> StatusCode {
        self.0.status_code()
    }
    pub fn location(&self) -> &str {
        self.0.location()
    }
}
impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        axum::response::IntoResponse::into_response(self.0).map(Body)
    }
}
impl<T: serde::Serialize> IntoResponse for super::Form<T> {
    fn into_response(self) -> Response {
        axum::response::IntoResponse::into_response(axum::Form(self.0)).map(Body)
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
