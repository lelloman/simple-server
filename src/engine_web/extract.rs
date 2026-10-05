//! Extractors that use the engine body's owned HTTP boundary.
use super::{Bytes, Request, RequestMetadata, StatusCode};
pub use crate::extract::{FromRequestParts, IntoRejectionResponse, Parts, RejectionResponse};
use std::{convert::Infallible, future::Future};

pub trait FromState<S> {
    fn from_state(state: &S) -> Self;
}
impl<T: Clone> FromState<T> for T {
    fn from_state(state: &T) -> Self {
        state.clone()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct State<T>(pub T);
#[derive(Clone, Copy, Debug)]
pub struct Query<T>(pub T);
#[derive(Clone, Copy, Debug)]
pub struct Json<T>(pub T);
#[derive(Clone, Debug)]
pub struct RawQuery(pub Option<String>);
#[derive(Clone, Debug)]
pub struct MatchedPath(String);
impl MatchedPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ConnectInfo<T>(pub T);
/// Per-request extension overriding the default 2 MiB extraction limit. Raw
/// request handlers and streaming bodies do not collect and are not limited by it.
#[derive(Clone, Copy, Debug)]
pub struct BodyLimit(pub usize);
#[doc(hidden)]
pub enum ViaBody {}
#[doc(hidden)]
pub enum ViaParts {}
/// Only the final argument of a typed handler may consume the request body.
pub trait FromRequest<S, M = ViaBody>: Sized {
    type Rejection: IntoRejectionResponse;
    fn from_request(
        request: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}
impl<S: Send + Sync, T: FromRequestParts<S>> FromRequest<S, ViaParts> for T {
    type Rejection = T::Rejection;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, _) = request.into_parts();
        T::from_request_parts(&mut parts, state).await
    }
}
impl<S: Sync, T: FromState<S> + Send> FromRequestParts<S> for State<T> {
    type Rejection = Infallible;
    async fn from_request_parts(_: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(T::from_state(state)))
    }
}
pub(super) fn rejection(status: StatusCode, text: impl Into<String>) -> RejectionResponse {
    let mut response = status.into_rejection_response();
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    *response.body_mut() = text.into().into_bytes();
    response
}
impl<S: Send + Sync, T: serde::de::DeserializeOwned + Send> FromRequestParts<S> for Query<T> {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let deserializer =
            serde_urlencoded::Deserializer::new(form_urlencoded::parse(query.as_bytes()));
        serde_path_to_error::deserialize(deserializer)
            .map(Self)
            .map_err(|error| {
                rejection(
                    StatusCode::BAD_REQUEST,
                    format!("Failed to deserialize query string: {error}"),
                )
            })
    }
}
impl<S: Sync> FromRequestParts<S> for RawQuery {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.uri.query().map(str::to_owned)))
    }
}
impl<S: Sync> FromRequestParts<S> for MatchedPath {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestMetadata>()
            .and_then(|m| m.matched_path.clone())
            .map(Self)
            .ok_or_else(|| {
                rejection(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Matched route unavailable",
                )
            })
    }
}
impl<S: Sync> FromRequestParts<S> for ConnectInfo<std::net::SocketAddr> {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestMetadata>()
            .and_then(|m| m.peer)
            .map(Self)
            .ok_or_else(|| {
                rejection(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing connection information",
                )
            })
    }
}
impl<S: Sync> FromRequest<S> for Request {
    type Rejection = Infallible;
    async fn from_request(request: Request, _: &S) -> Result<Self, Self::Rejection> {
        Ok(request)
    }
}
impl<S: Sync> FromRequest<S> for Bytes {
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, _: &S) -> Result<Self, Self::Rejection> {
        let limit = request
            .extensions()
            .get::<BodyLimit>()
            .map_or(2 * 1024 * 1024, |limit| limit.0);
        request.into_body().collect(limit).await.map_err(|error| {
            let mut cause: &(dyn std::error::Error + 'static) = &error;
            let mut limited = false;
            loop {
                if cause.is::<http_body_util::LengthLimitError>() {
                    limited = true;
                    break;
                }
                match cause.source() {
                    Some(source) => cause = source,
                    None => break,
                }
            }
            rejection(
                if limited {
                    StatusCode::PAYLOAD_TOO_LARGE
                } else {
                    StatusCode::BAD_REQUEST
                },
                format!("Failed to buffer the request body: {error}"),
            )
        })
    }
}
impl<S: Sync> FromRequest<S> for String {
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(request, state).await?;
        std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(|error| {
                rejection(
                    StatusCode::BAD_REQUEST,
                    format!("Request body didn't contain valid UTF-8: {error}"),
                )
            })
    }
}
impl<S: Send + Sync, T: serde::de::DeserializeOwned> FromRequest<S> for Json<T> {
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mime = request
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<mime::Mime>().ok());
        if !mime.is_some_and(|value| {
            value.type_() == "application"
                && (value.subtype() == "json"
                    || value.suffix().is_some_and(|suffix| suffix == "json"))
        }) {
            return Err(rejection(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected request with `Content-Type: application/json`",
            ));
        }
        let bytes = Bytes::from_request(request, state).await?;
        let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
        let value = serde_path_to_error::deserialize(&mut deserializer).map_err(
            |error: serde_path_to_error::Error<serde_json::Error>| {
                if error.inner().classify() == serde_json::error::Category::Data {
                    rejection(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!(
                            "Failed to deserialize the JSON body into the target type: {error}"
                        ),
                    )
                } else {
                    rejection(
                        StatusCode::BAD_REQUEST,
                        format!("Failed to parse the request body as JSON: {error}"),
                    )
                }
            },
        )?;
        deserializer.end().map_err(|error| {
            rejection(
                StatusCode::BAD_REQUEST,
                format!("Failed to parse the request body as JSON: {error}"),
            )
        })?;
        Ok(Self(value))
    }
}
impl<S: Send + Sync, T: serde::de::DeserializeOwned> FromRequest<S> for Option<Json<T>> {
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if request.headers().contains_key(http::header::CONTENT_TYPE) {
            Json::from_request(request, state).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
impl<S: Send + Sync, T: FromRequest<S>> FromRequest<S> for Result<T, T::Rejection> {
    type Rejection = Infallible;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Ok(T::from_request(request, state).await)
    }
}
macro_rules! deref {
    ($ty:ident) => {
        impl<T> std::ops::Deref for $ty<T> {
            type Target = T;
            fn deref(&self) -> &T {
                &self.0
            }
        }
        impl<T> std::ops::DerefMut for $ty<T> {
            fn deref_mut(&mut self) -> &mut T {
                &mut self.0
            }
        }
    };
}
deref!(State);
deref!(Json);
