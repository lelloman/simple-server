//! Application-owned request extraction without framework types.
//!
//! Implement [`FromRequestParts`] on an application value and request it with
//! [`Extract`] in a handler. Extraction runs on every use; this module does not
//! cache identities, authenticate callers, or reinterpret failures as anonymous.
//! Implement the trait on `Option<MyValue>` explicitly if optional extraction is
//! needed. Its policy remains application-owned, including which errors survive.
//!
//! The `extract` feature provides the contract without a runtime or framework.
//! With `http`, the framework adapter is implemented inside simple-server.

use std::{convert::Infallible, future::Future};

pub use http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts};

/// Buffered HTTP rejection, preserving status, headers, extensions and bytes.
/// Streaming successful responses are outside this extraction contract.
#[derive(Debug)]
pub struct RejectionResponse(http::Response<Vec<u8>>);

impl RejectionResponse {
    pub fn new(body: Vec<u8>) -> Self {
        Self(http::Response::new(body))
    }

    pub fn into_http(self) -> http::Response<Vec<u8>> {
        self.0
    }
}

impl From<http::Response<Vec<u8>>> for RejectionResponse {
    fn from(response: http::Response<Vec<u8>>) -> Self {
        Self(response)
    }
}

impl std::ops::Deref for RejectionResponse {
    type Target = http::Response<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RejectionResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "http")]
impl axum::response::IntoResponse for RejectionResponse {
    fn into_response(self) -> axum::response::Response {
        self.0.map(axum::body::Body::from)
    }
}

/// Render an extraction failure without depending on the HTTP framework.
pub trait IntoRejectionResponse {
    fn into_rejection_response(self) -> RejectionResponse;
}

impl IntoRejectionResponse for RejectionResponse {
    fn into_rejection_response(self) -> RejectionResponse {
        self
    }
}

impl IntoRejectionResponse for StatusCode {
    fn into_rejection_response(self) -> RejectionResponse {
        let mut response = RejectionResponse::new(Vec::new());
        *response.status_mut() = self;
        response
    }
}

impl IntoRejectionResponse for Infallible {
    fn into_rejection_response(self) -> RejectionResponse {
        match self {}
    }
}

/// Extract a value from the request head and application state.
///
/// The body is unavailable and remains untouched. Mutations to the head remain
/// visible to subsequent extractors and middleware. Implementations choose their
/// own state type, validation, side effects and error policy. Returned futures
/// must be `Send`; neither a runtime nor boxed futures are required by this API.
pub trait FromRequestParts<S>: Sized {
    type Rejection: IntoRejectionResponse;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

/// Handler argument for an application-owned extractor.
///
/// Destructure `Extract(value): Extract<MyValue>` to obtain the domain value.
/// For an explicit optional policy, use `Extract<Option<MyValue>>`, not
/// `Option<Extract<MyValue>>`: this wrapper never silently discards rejections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extract<T>(pub T);

impl<T> Extract<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<S, T: FromRequestParts<S>> FromRequestParts<S> for Extract<T> {
    type Rejection = T::Rejection;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let future = T::from_request_parts(parts, state);
        async move { future.await.map(Self) }
    }
}

#[cfg(feature = "http")]
impl<S, T> axum::extract::FromRequestParts<S> for Extract<T>
where
    S: Send + Sync,
    T: FromRequestParts<S>,
{
    // No Axum type in the public associated rejection type.
    type Rejection = RejectionResponse;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        T::from_request_parts(parts, state)
            .await
            .map(Self)
            .map_err(IntoRejectionResponse::into_rejection_response)
    }
}
