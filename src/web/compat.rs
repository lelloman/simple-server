//! Explicit backend boundaries for incremental protocol migration.
use super::{Body, Response, Router};

pub fn into_axum_router<S>(router: Router<S>) -> axum::Router<S> {
    router.inner
}

/// Convert a legacy protocol response without buffering its body.
pub fn response(response: impl axum::response::IntoResponse) -> Response {
    response.into_response().map(Body)
}

#[cfg(feature = "multipart")]
pub struct Multipart(axum::extract::Multipart);
#[cfg(feature = "multipart")]
impl<S: Send + Sync> super::FromRequest<S> for Multipart {
    type Rejection = super::RejectionResponse;
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
    pub async fn next_field(
        &mut self,
    ) -> Result<Option<axum::extract::multipart::Field<'_>>, axum::extract::multipart::MultipartError>
    {
        self.0.next_field().await
    }
}

#[cfg(feature = "ws")]
pub struct WebSocketUpgrade(axum::extract::WebSocketUpgrade);
#[cfg(feature = "ws")]
impl<S: Send + Sync> super::FromRequestParts<S> for WebSocketUpgrade {
    type Rejection = super::RejectionResponse;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRequestParts;
        axum::extract::WebSocketUpgrade::from_request_parts(parts, state)
            .await
            .map(Self)
            .map_err(|error| super::extract::rejection(error.status(), error.body_text()))
    }
}
#[cfg(feature = "ws")]
impl WebSocketUpgrade {
    pub fn on_upgrade<C, F>(self, callback: C) -> Response
    where
        C: FnOnce(axum::extract::ws::WebSocket) -> F + Send + 'static,
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        response(self.0.on_upgrade(callback))
    }
}

/// Preserve the existing tracing observer during a middleware migration.
#[cfg(feature = "http-tracing")]
pub async fn trace_with_observer<F, Fut, O>(
    request: super::Request,
    observer: O,
    next: F,
) -> Response
where
    F: FnOnce(super::Request) -> Fut,
    Fut: std::future::Future<Output = Response>,
    O: crate::http_tracing::Observer,
{
    crate::http_tracing::trace_with_observer(
        request.map(|body| body.0),
        observer,
        |request| async move {
            next(super::service::request(request))
                .await
                .map(|body| body.0)
        },
    )
    .await
    .map(Body)
}
