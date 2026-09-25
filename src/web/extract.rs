pub use super::Request;
pub use crate::extract::{FromRequestParts, IntoRejectionResponse, Parts, RejectionResponse};
use std::{convert::Infallible, future::Future};

/// Select application substate without a framework-specific state trait.
pub trait FromState<S> {
    fn from_state(state: &S) -> Self;
}

impl<T: Clone> FromState<T> for T {
    fn from_state(state: &T) -> Self {
        state.clone()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct State<T>(pub T);
#[derive(Debug, Clone, Copy)]
pub struct Path<T>(pub T);
#[derive(Debug, Clone, Copy)]
pub struct Query<T>(pub T);
#[derive(Debug, Clone, Copy)]
pub struct Json<T>(pub T);

// Public only to permit inference of the body-vs-parts coherence marker. These
// contain no framework types and applications normally never name them.
#[doc(hidden)]
pub enum ViaBody {}
#[doc(hidden)]
pub enum ViaParts {}

/// A body-consuming extractor, permitted only as the final handler argument.
/// Head-only extractors automatically implement this through a separate marker.
pub trait FromRequest<S, M = ViaBody>: Sized {
    type Rejection: IntoRejectionResponse;
    fn from_request(
        request: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

impl<S, T> FromRequest<S, ViaParts> for T
where
    S: Send + Sync,
    T: FromRequestParts<S>,
{
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

pub(super) fn rejection(status: http::StatusCode, text: String) -> RejectionResponse {
    let mut response = status.into_rejection_response();
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    *response.body_mut() = text.into_bytes();
    response
}

macro_rules! head_extractor {
    ($name:ident) => {
        impl<S, T> FromRequestParts<S> for $name<T>
        where
            S: Send + Sync,
            T: serde::de::DeserializeOwned + Send,
        {
            type Rejection = RejectionResponse;
            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                <axum::extract::$name<T> as axum::extract::FromRequestParts<S>>::from_request_parts(
                    parts, state,
                )
                .await
                .map(|value| Self(value.0))
                .map_err(|error| rejection(error.status(), error.body_text()))
            }
        }
    };
}
head_extractor!(Path);
head_extractor!(Query);

impl<S, T> FromRequest<S> for Json<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        <axum::Json<T> as axum::extract::FromRequest<S>>::from_request(
            request.map(|body| body.0),
            state,
        )
        .await
        .map(|value| Self(value.0))
        .map_err(|error| rejection(error.status(), error.body_text()))
    }
}

impl<S: Sync> FromRequest<S> for Request {
    type Rejection = Infallible;
    async fn from_request(request: Request, _: &S) -> Result<Self, Self::Rejection> {
        Ok(request)
    }
}

macro_rules! body_extractor {
    ($ty:ty) => {
        impl<S: Send + Sync> FromRequest<S> for $ty {
            type Rejection = RejectionResponse;
            async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
                <$ty as axum::extract::FromRequest<S>>::from_request(
                    request.map(|body| body.0),
                    state,
                )
                .await
                .map_err(|error| rejection(error.status(), error.body_text()))
            }
        }
    };
}
body_extractor!(String);
body_extractor!(bytes::Bytes);

impl<S: Sync> FromRequestParts<S> for http::HeaderMap {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.headers.clone())
    }
}

// Result captures a rejection explicitly, for applications that need to render
// a custom error envelope. It does not suppress the error or turn it anonymous.
impl<S, T> FromRequestParts<S> for Result<T, T::Rejection>
where
    S: Send + Sync,
    T: FromRequestParts<S>,
{
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(T::from_request_parts(parts, state).await)
    }
}

impl<S, T> FromRequest<S> for Result<T, T::Rejection>
where
    S: Send + Sync,
    T: FromRequest<S>,
{
    type Rejection = Infallible;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Ok(T::from_request(request, state).await)
    }
}

/// Request-scoped value installed by a Tower layer or earlier middleware.
#[derive(Debug, Clone, Copy)]
pub struct Extension<T>(pub T);
#[derive(Debug, Clone, Copy)]
pub struct ConnectInfo<T>(pub T);
/// A bounded route template, never the concrete URL or query string.
#[derive(Debug, Clone)]
pub struct MatchedPath(pub(super) std::sync::Arc<str>);
impl MatchedPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<S: Send + Sync, T: Clone + Send + Sync + 'static> FromRequestParts<S> for Extension<T> {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        <axum::Extension<T> as axum::extract::FromRequestParts<S>>::from_request_parts(parts, state)
            .await
            .map(|value| Self(value.0))
            .map_err(|error| rejection(error.status(), error.body_text()))
    }
}
impl<S: Sync, T: Clone + Send + Sync + 'static> FromRequestParts<S> for ConnectInfo<T> {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            rejection(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Missing connection information".into(),
            )
        })
    }
}
impl<S: Sync> FromRequestParts<S> for MatchedPath {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            rejection(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Matched route unavailable".into(),
            )
        })
    }
}
impl<T: Clone + Send + Sync + 'static, I> tower_layer::Layer<I> for Extension<T> {
    type Service = AddExtension<I, T>;
    fn layer(&self, inner: I) -> Self::Service {
        AddExtension {
            inner,
            value: self.0.clone(),
        }
    }
}
#[derive(Clone)]
pub struct AddExtension<I, T> {
    inner: I,
    value: T,
}
impl<I, T: Clone + Send + Sync + 'static> tower_service::Service<Request> for AddExtension<I, T>
where
    I: tower_service::Service<Request>,
{
    type Response = I::Response;
    type Error = I::Error;
    type Future = I::Future;
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, mut request: Request) -> Self::Future {
        request.extensions_mut().insert(self.value.clone());
        self.inner.call(request)
    }
}

// Only absence is optional here; authentication/extractor errors are not swallowed.
impl<S: Sync, T: Clone + Send + Sync + 'static> FromRequestParts<S> for Option<Extension<T>> {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.extensions.get::<T>().cloned().map(Extension))
    }
}
impl<S: Sync> FromRequestParts<S> for http::Uri {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts.uri.clone())
    }
}

/// URL-encoded forms: GET/HEAD read the query; other methods read the body.
#[derive(Debug, Clone, Copy)]
pub struct Form<T>(pub T);
impl<S, T> FromRequest<S> for Form<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        <axum::Form<T> as axum::extract::FromRequest<S>>::from_request(
            request.map(|body| body.0),
            state,
        )
        .await
        .map(|value| Self(value.0))
        .map_err(|error| rejection(error.status(), error.body_text()))
    }
}

/// Extract the cookie jar installed by Tower's CookieManagerLayer.
/// Cookie policy, parsing and response deltas remain owned by tower-cookies.
#[cfg(feature = "tower-cookies")]
impl<S: Send + Sync> FromRequestParts<S> for tower_cookies::Cookies {
    type Rejection = RejectionResponse;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            rejection(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Can't extract cookies. Is `CookieManagerLayer` enabled?".to_owned(),
            )
        })
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T> std::ops::DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

#[cfg(any(feature = "multipart", feature = "multipart-owned"))]
pub use super::multipart;
#[cfg(feature = "multipart")]
pub use super::multipart::Multipart;
#[cfg(feature = "multipart-owned")]
pub use super::multipart::OwnedMultipart;

/// Preserve the raw, undecoded query, including the distinction between no
/// query and an explicitly empty query.
#[derive(Debug, Clone)]
pub struct RawQuery(pub Option<String>);
impl<S: Send + Sync> FromRequestParts<S> for RawQuery {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.uri.query().map(str::to_owned)))
    }
}

/// Optional JSON is absent only when Content-Type is absent. Invalid declared
/// JSON, unsupported media types and body-limit errors still reject the request.
impl<S, T> FromRequest<S> for Option<Json<T>>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = RejectionResponse;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        <Option<axum::Json<T>> as axum::extract::FromRequest<S>>::from_request(
            request.map(|body| body.0),
            state,
        )
        .await
        .map(|value| value.map(|value| Json(value.0)))
        .map_err(|error| rejection(error.status(), error.body_text()))
    }
}
