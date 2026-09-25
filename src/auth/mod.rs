//! Authentication and authorization with application-owned identities and errors.
//!
//! Verifiers run first; access checks run in order and stop on the first error.
//! No identity is returned or propagated until every check succeeds. Callers own
//! credential precedence (optionally through CredentialSources), session/token
//! validation, revocation, resource policy, CSRF, error responses and audit side
//! effects. There is no implicit anonymous
//! access, caching, retry, timeout, logging, or spawned work.
/// Framework-independent cookie values for issuance/expiration. Parsing policy
/// lives in CookieCredential; applications own security attributes and headers.
#[cfg(feature = "auth-cookies")]
pub use ::cookie::{Cookie, SameSite};
mod cookie;
mod credential;
mod selection;
pub use cookie::{
    CookieCredential, CookieCredentialError, CookieValue, InvalidCookieName, RepeatedCookies,
};
pub use selection::{
    Authentication, CredentialAuthError, CredentialSelectionError, CredentialSource,
    CredentialSources, MalformedCredentials, SelectedCredential,
};
mod layer;
pub use credential::{Credential, CredentialError, HeaderCredential, RepeatedHeaders, SchemeCase};
pub use http::{HeaderMap, HeaderName};
pub use layer::{AuthLayer, AuthService, Identity};
use std::{future::Future, pin::Pin, sync::Arc};

/// Borrowing future for asynchronous verification or resource checks.
pub type AuthFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type Verifier<C, P, E> = dyn Fn(&C) -> Result<P, E> + Send + Sync;
type Check<C, P, E> = dyn Fn(&P, &C) -> Result<(), E> + Send + Sync;
type AsyncVerifier<C, P, E> = dyn for<'a> Fn(&'a C) -> AuthFuture<'a, Result<P, E>> + Send + Sync;
type AsyncCheck<C, P, E> =
    dyn for<'a> Fn(&'a P, &'a C) -> AuthFuture<'a, Result<(), E>> + Send + Sync;

/// Synchronous access flow. The context may be headers, a resource/action tuple,
/// or any application type. Construction always requires an explicit verifier.
/// A verifier-only flow grants access to any identity accepted by that verifier.
/// Use `with_check` to add route/resource restrictions; all checks must pass.
pub struct Access<C, P, E> {
    verify: Arc<Verifier<C, P, E>>,
    checks: Vec<Arc<Check<C, P, E>>>,
}
impl<C, P, E> Clone for Access<C, P, E> {
    fn clone(&self) -> Self {
        Self {
            verify: self.verify.clone(),
            checks: self.checks.clone(),
        }
    }
}
impl<C, P, E> Access<C, P, E> {
    pub fn new(verify: impl Fn(&C) -> Result<P, E> + Send + Sync + 'static) -> Self {
        Self {
            verify: Arc::new(verify),
            checks: Vec::new(),
        }
    }
    pub fn with_check(
        mut self,
        check: impl Fn(&P, &C) -> Result<(), E> + Send + Sync + 'static,
    ) -> Self {
        self.checks.push(Arc::new(check));
        self
    }
    pub fn evaluate(&self, context: &C) -> Result<P, E> {
        let principal = (self.verify)(context)?;
        self.authorize(&principal, context)?;
        Ok(principal)
    }
    /// Apply this flow's access checks to an identity already verified by the
    /// caller. This does NOT authenticate that identity or refresh its privileges.
    pub fn authorize(&self, principal: &P, context: &C) -> Result<(), E> {
        for check in &self.checks {
            check(principal, context)?;
        }
        Ok(())
    }
}

/// Async counterpart for database/session/provider verification and resource
/// checks. Futures borrow the request context; dropping evaluation cancels the
/// current future and never starts subsequent checks. Already committed side
/// effects remain the application's responsibility. Clone shares callbacks, not
/// identities or request state.
pub struct AsyncAccess<C, P, E> {
    verify: Arc<AsyncVerifier<C, P, E>>,
    checks: Vec<Arc<AsyncCheck<C, P, E>>>,
}
impl<C, P, E> Clone for AsyncAccess<C, P, E> {
    fn clone(&self) -> Self {
        Self {
            verify: self.verify.clone(),
            checks: self.checks.clone(),
        }
    }
}
impl<C, P, E> AsyncAccess<C, P, E> {
    pub fn new(
        verify: impl for<'a> Fn(&'a C) -> AuthFuture<'a, Result<P, E>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            verify: Arc::new(verify),
            checks: Vec::new(),
        }
    }
    pub fn with_check(
        mut self,
        check: impl for<'a> Fn(&'a P, &'a C) -> AuthFuture<'a, Result<(), E>> + Send + Sync + 'static,
    ) -> Self {
        self.checks.push(Arc::new(check));
        self
    }
    pub async fn evaluate(&self, context: &C) -> Result<P, E> {
        let principal = (self.verify)(context).await?;
        self.authorize(&principal, context).await?;
        Ok(principal)
    }
    /// Check an already verified identity; see [`Access::authorize`].
    pub async fn authorize(&self, principal: &P, context: &C) -> Result<(), E> {
        for check in &self.checks {
            check(principal, context).await?;
        }
        Ok(())
    }
}
