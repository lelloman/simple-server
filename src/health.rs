//! Optional liveness and readiness probes with application-owned checks and responses.
//!
//! Checks run afresh, sequentially, and stop at the first failure. No tasks, timers,
//! logging, database connections, or lifecycle transitions are installed implicitly.
//! Mount [`Endpoint`](crate::health::Endpoint) behind the application's GET/HEAD routing and access controls;
//! it deliberately leaves method dispatch, paths, and response policy to that router.

use http::{Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower_service::Service;

type CheckFuture<E, T> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

/// A named application check. Wrap the callback in your own timeout if required.
pub struct Check<E, T = ()> {
    name: Arc<str>,
    run: Arc<dyn Fn() -> CheckFuture<E, T> + Send + Sync>,
}

impl<E, T> Clone for Check<E, T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            run: self.run.clone(),
        }
    }
}

impl<E, T> Check<E, T> {
    /// The callback creates a fresh future on every probe invocation.
    pub fn new<F, Fut>(name: impl Into<Arc<str>>, check: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        Self {
            name: name.into(),
            run: Arc::new(move || Box::pin(check())),
        }
    }

    /// Evaluate a named check while retaining its success payload. Useful for
    /// aggregate health reports whose detailed body must survive either outcome.
    /// No short-circuiting is imposed inside an application-owned aggregate check.
    pub async fn run(&self) -> Result<T, CheckFailure<E>> {
        (self.run)().await.map_err(|error| CheckFailure {
            name: self.name.clone(),
            error,
        })
    }
}

/// The first failed check, retaining the application's original error.
#[derive(Debug)]
pub struct CheckFailure<E> {
    pub name: Arc<str>,
    pub error: E,
}

/// An explicit liveness probe or an ordered collection of readiness checks.
pub struct Probe<E> {
    checks: Vec<Check<E>>,
}

impl<E> Clone for Probe<E> {
    fn clone(&self) -> Self {
        Self {
            checks: self.checks.clone(),
        }
    }
}

impl Probe<Infallible> {
    /// Process liveness only. This does not establish application readiness.
    pub fn liveness() -> Self {
        Self { checks: Vec::new() }
    }
}

impl<E> Probe<E> {
    /// Readiness requires at least one explicit application check.
    pub fn readiness(first: Check<E>) -> Self {
        Self {
            checks: vec![first],
        }
    }

    /// Append a check. Later checks do not run if an earlier check fails.
    pub fn with_check(mut self, check: Check<E>) -> Self {
        self.checks.push(check);
        self
    }

    /// Evaluate without HTTP. Dropping this future cancels the currently awaited
    /// check; callbacks must manage any detached work or blocking operations themselves.
    pub async fn run(&self) -> Result<(), CheckFailure<E>> {
        for check in &self.checks {
            check.run().await?;
        }
        Ok(())
    }

    /// Adapt to Tower using an application-owned response renderer. Errors are
    /// passed to the renderer, never automatically serialized or logged.
    pub fn endpoint<R, Out>(self, render: R) -> Endpoint<E, R>
    where
        R: Fn(Result<(), CheckFailure<E>>) -> Response<Out>,
    {
        Endpoint {
            probe: self,
            render,
        }
    }
}

/// Framework-independent HTTP adapter. Each call evaluates the probe independently.
///
/// `poll_ready` reports Tower capacity, not application health. The service accepts
/// every method; install GET/HEAD routing outside it. Request bodies are not read.
pub struct Endpoint<E, R> {
    probe: Probe<E>,
    render: R,
}

impl<E, R: Clone> Clone for Endpoint<E, R> {
    fn clone(&self) -> Self {
        Self {
            probe: self.probe.clone(),
            render: self.render.clone(),
        }
    }
}

impl<E, R, B, Out> Service<Request<B>> for Endpoint<E, R>
where
    E: Send + 'static,
    R: Fn(Result<(), CheckFailure<E>>) -> Response<Out> + Clone + Send + 'static,
    Out: 'static,
{
    type Response = Response<Out>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request<B>) -> Self::Future {
        let probe = self.probe.clone();
        let render = self.render.clone();
        Box::pin(async move {
            let result = probe.run().await;
            Ok(render(result))
        })
    }
}
