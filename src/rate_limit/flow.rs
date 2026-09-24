use std::{future::Future, pin::Pin, sync::Arc};

/// Accepted work may own concurrency/reservation guards. Drop releases guards,
/// never refunds consumed rate tokens. Guards are deliberately not clonable.
#[must_use = "hold the admission for the lifetime of the admitted work"]
#[derive(Default)]
pub struct Admission {
    guards: Vec<Box<dyn Send + Sync>>,
}
impl Admission {
    /// Explicit unconditional acceptance, also used for application exemptions.
    pub fn unrestricted() -> Self {
        Self::default()
    }
    pub fn holding(guard: impl Send + Sync + 'static) -> Self {
        Self {
            guards: vec![Box::new(guard)],
        }
    }
    fn append(&mut self, mut other: Self) {
        self.guards.append(&mut other.guards);
    }
}
impl std::fmt::Debug for Admission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Admission")
            .field("guards", &self.guards.len())
            .finish()
    }
}
pub type AdmissionFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type Check<C, E> = dyn Fn(&C) -> Result<Admission, E> + Send + Sync;
type AsyncCheck<C, E> =
    dyn for<'a> Fn(&'a C) -> AdmissionFuture<'a, Result<Admission, E>> + Send + Sync;

/// Ordered, consuming checks. A later rejection releases prior guards but does
/// NOT roll back prior charges. For atomic multi-budget or durable operations,
/// supply one application callback performing its transaction in full.
pub struct Policy<C, E> {
    checks: Vec<Arc<Check<C, E>>>,
}
impl<C, E> Clone for Policy<C, E> {
    fn clone(&self) -> Self {
        Self {
            checks: self.checks.clone(),
        }
    }
}
impl<C, E> Policy<C, E> {
    pub fn new(check: impl Fn(&C) -> Result<Admission, E> + Send + Sync + 'static) -> Self {
        Self {
            checks: vec![Arc::new(check)],
        }
    }
    pub fn with_check(
        mut self,
        check: impl Fn(&C) -> Result<Admission, E> + Send + Sync + 'static,
    ) -> Self {
        self.checks.push(Arc::new(check));
        self
    }
    pub fn evaluate(&self, context: &C) -> Result<Admission, E> {
        let mut admission = Admission::unrestricted();
        for check in &self.checks {
            admission.append(check(context)?);
        }
        Ok(admission)
    }
}

/// Async storage/policy extension point. Backend errors stay application-owned,
/// distinct from quota rejection. Cancellation drops acquired guards and the
/// current future; no worker is spawned. Committed side effects are not undone.
pub struct AsyncPolicy<C, E> {
    checks: Vec<Arc<AsyncCheck<C, E>>>,
}
impl<C, E> Clone for AsyncPolicy<C, E> {
    fn clone(&self) -> Self {
        Self {
            checks: self.checks.clone(),
        }
    }
}
impl<C, E> AsyncPolicy<C, E> {
    pub fn new(
        check: impl for<'a> Fn(&'a C) -> AdmissionFuture<'a, Result<Admission, E>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            checks: vec![Arc::new(check)],
        }
    }
    pub fn with_check(
        mut self,
        check: impl for<'a> Fn(&'a C) -> AdmissionFuture<'a, Result<Admission, E>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.checks.push(Arc::new(check));
        self
    }
    pub async fn evaluate(&self, context: &C) -> Result<Admission, E> {
        let mut admission = Admission::unrestricted();
        for check in &self.checks {
            admission.append(check(context).await?);
        }
        Ok(admission)
    }
}
impl<C: 'static, E: Send + 'static> AsyncPolicy<C, E> {
    pub fn from_sync(policy: Policy<C, E>) -> Self {
        Self::new(move |context| {
            let result = policy.evaluate(context);
            Box::pin(std::future::ready(result))
        })
    }
}
