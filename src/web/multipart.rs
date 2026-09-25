//! Streaming multipart uploads with simple-server-owned fields and errors.
//!
//! `Multipart` borrows each field, enforcing one live field at compile time.
//! `OwnedMultipart` (feature `multipart-owned`) returns owned fields and enforces
//! exclusivity at runtime. Both preserve the configured extractor body limit.
//! Fields are streamed; only `bytes` and `text` collect a complete field.
//! Application-specific file/metadata limits and storage policy remain with callers.
use super::{Bytes, IntoRejectionResponse, IntoResponse, RejectionResponse, Response};
use std::{error::Error, fmt};

/// A parser/body-read failure without backend types in its public contract.
#[derive(Debug)]
pub struct MultipartError {
    status: http::StatusCode,
    text: String,
    message: String,
    source: Box<dyn Error + Send + Sync>,
}
impl MultipartError {
    /// HTTP rejection status, including 413 for body-limit failures.
    pub fn status(&self) -> http::StatusCode {
        self.status
    }
    /// Detailed rejection text. Display retains the parser's diagnostic summary.
    pub fn body_text(&self) -> String {
        self.text.clone()
    }
    #[cfg(feature = "multipart")]
    fn borrowed(error: axum::extract::multipart::MultipartError) -> Self {
        Self {
            status: error.status(),
            text: error.body_text(),
            message: error.to_string(),
            source: Box::new(error),
        }
    }
    #[cfg(feature = "multipart-owned")]
    fn owned(error: axum_extra::extract::multipart::MultipartError) -> Self {
        Self {
            status: error.status(),
            text: error.body_text(),
            message: error.to_string(),
            source: Box::new(error),
        }
    }
}
impl fmt::Display for MultipartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl Error for MultipartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
impl IntoRejectionResponse for MultipartError {
    fn into_rejection_response(self) -> RejectionResponse {
        super::extract::rejection(self.status, self.text)
    }
}
impl IntoResponse for MultipartError {
    fn into_response(self) -> Response {
        self.into_rejection_response().into_response()
    }
}

/// Multipart reader with statically exclusive borrowed fields.
#[cfg(feature = "multipart")]
pub struct Multipart(axum::extract::Multipart);
#[cfg(feature = "multipart")]
impl<S: Send + Sync> super::FromRequest<S> for Multipart {
    type Rejection = RejectionResponse;
    async fn from_request(request: super::Request, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRequest;
        axum::extract::Multipart::from_request(request.map(|body| body.0), state)
            .await
            .map(Self)
            .map_err(|error| super::extract::rejection(error.status(), error.body_text()))
    }
}
#[cfg(feature = "multipart")]
impl Multipart {
    pub async fn next_field(&mut self) -> Result<Option<Field<'_>>, MultipartError> {
        self.0
            .next_field()
            .await
            .map(|field| field.map(Field))
            .map_err(MultipartError::borrowed)
    }
}
/// One field borrowed from a multipart reader. Dropping it permits the next field.
#[cfg(feature = "multipart")]
pub struct Field<'a>(axum::extract::multipart::Field<'a>);

/// Multipart reader whose owned fields enforce exclusivity at runtime.
#[cfg(feature = "multipart-owned")]
pub struct OwnedMultipart(axum_extra::extract::Multipart);
#[cfg(feature = "multipart-owned")]
impl<S: Send + Sync> super::FromRequest<S> for OwnedMultipart {
    type Rejection = RejectionResponse;
    async fn from_request(request: super::Request, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRequest;
        axum_extra::extract::Multipart::from_request(request.map(|body| body.0), state)
            .await
            .map(Self)
            .map_err(|error| super::extract::rejection(error.status(), error.body_text()))
    }
}
#[cfg(feature = "multipart-owned")]
impl OwnedMultipart {
    pub async fn next_field(&mut self) -> Result<Option<OwnedField>, MultipartError> {
        self.0
            .next_field()
            .await
            .map(|field| field.map(OwnedField))
            .map_err(MultipartError::owned)
    }
}
/// An owned field. Drop or fully consume it before requesting another field.
#[cfg(feature = "multipart-owned")]
pub struct OwnedField(axum_extra::extract::multipart::Field);

macro_rules! field {
    ($ty:ty, $error:ident) => {
        impl $ty {
            pub fn name(&self) -> Option<&str> {
                self.0.name()
            }
            pub fn file_name(&self) -> Option<&str> {
                self.0.file_name()
            }
            pub fn content_type(&self) -> Option<&str> {
                self.0.content_type()
            }
            pub fn headers(&self) -> &http::HeaderMap {
                self.0.headers()
            }
            /// Collect the field, respecting the enclosing request body limit.
            pub async fn bytes(self) -> Result<Bytes, MultipartError> {
                self.0.bytes().await.map_err(MultipartError::$error)
            }
            /// Decode the field using the parser's text/charset behavior.
            pub async fn text(self) -> Result<String, MultipartError> {
                self.0.text().await.map_err(MultipartError::$error)
            }
            /// Read the next chunk without collecting the entire field.
            pub async fn chunk(&mut self) -> Result<Option<Bytes>, MultipartError> {
                self.0.chunk().await.map_err(MultipartError::$error)
            }
        }
        impl futures_util::Stream for $ty {
            type Item = Result<Bytes, MultipartError>;
            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Self::Item>> {
                std::pin::Pin::new(&mut self.get_mut().0)
                    .poll_next(cx)
                    .map(|value| value.map(|result| result.map_err(MultipartError::$error)))
            }
        }
    };
}
#[cfg(feature = "multipart")]
field!(Field<'_>, borrowed);
#[cfg(feature = "multipart-owned")]
field!(OwnedField, owned);
