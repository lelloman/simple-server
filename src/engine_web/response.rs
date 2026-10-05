//! Common response conversions without a source HTTP framework.
use super::{Body, HeaderMap, Json, Response, StatusCode};
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
