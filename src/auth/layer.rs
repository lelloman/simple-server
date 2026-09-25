use super::{
    AsyncAccess, AuthFuture, Authentication, CredentialAuthError, CredentialSelectionError,
    CredentialSource, CredentialSources, SelectedCredential,
};
use http::{Request, Response, request::Parts};
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

/// Identity established by this layer after all access checks succeed. Public
/// construction is intentionally absent. This is a request-local result, not a
/// durable authorization grant; resource-specific checks may still be required.
#[derive(Clone)]
pub struct Identity<P> {
    principal: P,
    source: Option<CredentialSource>,
}
impl<P> Identity<P> {
    /// None for the legacy application-owned Parts verifier; Some for credential selection.
    pub fn source(&self) -> Option<&CredentialSource> {
        self.source.as_ref()
    }
    pub fn principal(&self) -> &P {
        &self.principal
    }
    pub fn into_principal(self) -> P {
        self.principal
    }
}

enum Gate<P, E> {
    Legacy(AsyncAccess<Parts, P, E>),
    Credentials(AsyncAccess<Parts, Option<Identity<P>>, E>),
}
impl<P, E> Clone for Gate<P, E> {
    fn clone(&self) -> Self {
        match self {
            Self::Legacy(a) => Self::Legacy(a.clone()),
            Self::Credentials(a) => Self::Credentials(a.clone()),
        }
    }
}

/// Framework-independent HTTP gate using `http` and Tower. Mount explicitly on
/// protected routes, or choose Optional for explicit anonymous access when no
/// credential is supplied. Public routes can stay outside this layer. It never
/// reads or buffers the body, logs credentials, redirects, or chooses status codes.
/// Rendering failures, including provider outages, is application-owned.
/// A pre-existing `Identity<P>` is removed before verification to prevent stale
/// identity reuse when stacking gates. Other request extensions are preserved.
pub struct AuthLayer<P, E, R> {
    access: Gate<P, E>,
    reject: R,
}
impl<P, E, R: Clone> Clone for AuthLayer<P, E, R> {
    fn clone(&self) -> Self {
        Self {
            access: self.access.clone(),
            reject: self.reject.clone(),
        }
    }
}
impl<P, E, R> AuthLayer<P, E, R> {
    pub fn new(access: AsyncAccess<Parts, P, E>, reject: R) -> Self {
        Self {
            access: Gate::Legacy(access),
            reject,
        }
    }
}

impl<P: Send + Sync + 'static, E: 'static, R> AuthLayer<P, CredentialAuthError<E>, R> {
    /// Select a header/cookie credential, then invoke the supplied verifier and
    /// access checks exactly once. Only absent credentials may pass anonymously.
    /// The layer adds source metadata beside the application principal; it does
    /// not attach the selected secret to request extensions.
    /// For policies needing full request Parts, use CredentialSources::extract
    /// from an application-owned verifier with AuthLayer::new instead.
    ///
    /// ```
    /// use simple_server::auth::*;
    /// let sources = CredentialSources::header(
    ///     HeaderCredential::new(http::header::AUTHORIZATION)
    ///         .with_scheme("Bearer", SchemeCase::AsciiInsensitive),
    /// ).or_cookie(CookieCredential::new("session_token")?);
    /// // Supply your real database/provider verifier in place of this fixture.
    /// let access = AsyncAccess::new(|credential: &SelectedCredential| {
    ///     Box::pin(async move {
    ///         if credential.expose() == "fixture-token" { Ok(42u64) }
    ///         else { Err(()) }
    ///     })
    /// });
    /// let layer = AuthLayer::credentials(sources, Authentication::Required, access,
    ///     |_: CredentialAuthError<()>| http::Response::builder()
    ///         .status(401).body("unauthorized").unwrap());
    /// # let _ = layer;
    /// # Ok::<(), InvalidCookieName>(())
    /// ```
    pub fn credentials(
        sources: CredentialSources,
        authentication: Authentication,
        access: AsyncAccess<SelectedCredential, P, E>,
        reject: R,
    ) -> Self {
        let access = AsyncAccess::new(move |parts: &Parts| {
            let selected = sources.extract(&parts.headers);
            let access = access.clone();
            Box::pin(async move {
                let selected = match selected {
                    Ok(selected) => selected,
                    Err(CredentialSelectionError::Missing)
                        if matches!(authentication, Authentication::Optional) =>
                    {
                        return Ok(None);
                    }
                    Err(error) => return Err(CredentialAuthError::Selection(error)),
                };
                let principal = access
                    .evaluate(&selected)
                    .await
                    .map_err(CredentialAuthError::Access)?;
                Ok(Some(Identity {
                    principal,
                    source: Some(selected.source().clone()),
                }))
            })
        });
        Self {
            access: Gate::Credentials(access),
            reject,
        }
    }
}
impl<S, P, E, R: Clone> Layer<S> for AuthLayer<P, E, R> {
    type Service = AuthService<S, P, E, R>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            access: self.access.clone(),
            reject: self.reject.clone(),
        }
    }
}
pub struct AuthService<S, P, E, R> {
    inner: S,
    access: Gate<P, E>,
    reject: R,
}
impl<S: Clone, P, E, R: Clone> Clone for AuthService<S, P, E, R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            access: self.access.clone(),
            reject: self.reject.clone(),
        }
    }
}
impl<S, P, E, R, B, Out> Service<Request<B>> for AuthService<S, P, E, R>
where
    S: Service<Request<B>, Response = Response<Out>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Send,
    P: Clone + Send + Sync + 'static,
    E: Send + 'static,
    R: Fn(E) -> Response<Out> + Clone + Send + 'static,
    B: Send + 'static,
    Out: 'static,
{
    type Response = Response<Out>;
    type Error = S::Error;
    type Future = AuthFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: Request<B>) -> Self::Future {
        // Move the instance whose readiness was polled, never call its unready clone.
        let replacement = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, replacement);
        let access = self.access.clone();
        let reject = self.reject.clone();
        Box::pin(async move {
            let (mut parts, body) = request.into_parts();
            parts.extensions.remove::<Identity<P>>();
            let result = match access {
                Gate::Legacy(access) => access.evaluate(&parts).await.map(|principal| {
                    Some(Identity {
                        principal,
                        source: None,
                    })
                }),
                Gate::Credentials(access) => access.evaluate(&parts).await,
            };
            match result {
                Ok(identity) => {
                    if let Some(identity) = identity {
                        parts.extensions.insert(identity);
                    }
                    inner.call(Request::from_parts(parts, body)).await
                }
                Err(error) => Ok(reject(error)),
            }
        })
    }
}
