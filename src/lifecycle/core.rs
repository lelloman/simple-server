use std::{collections::BTreeSet, error::Error, fmt, future::Future, time::Duration};

use super::clock::{Instant, sleep_until};
pub use crate::shutdown::Shutdown;
use futures_util::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use std::{pin::pin, task::Poll};

/// An application or service error, preserving its original source.
pub type BoxError = Box<dyn Error + Send + Sync>;

/// A supported operating-system shutdown signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// SIGINT on Unix, Ctrl+C on Windows.
    Interrupt,
    /// SIGTERM (Unix only).
    Terminate,
}

/// Why the coordinator began shutting down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownReason {
    /// An application called [`Shutdown::request`].
    Requested,
    /// A caller-provided trigger completed normally.
    Triggered,
    /// An operating-system signal arrived.
    Signal(Signal),
    /// A long-running service returned before shutdown was requested.
    ServiceExited(String),
    /// The supplied trigger failed.
    TriggerFailed,
}

/// Explicit budget for draining all services and then performing final cleanup.
#[derive(Clone, Copy, Debug)]
pub struct ShutdownOptions {
    /// Starts when the coordinator observes shutdown. Zero permits no drain wait.
    pub grace_period: Duration,
}

/// A rejected lifecycle configuration.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigurationError {
    /// Names must be unique, so failure reports remain unambiguous.
    DuplicateService(String),
    /// At least one long-running service must be registered.
    NoServices,
    /// The requested budget cannot be represented by the runtime clock.
    InvalidGracePeriod,
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateService(name) => write!(f, "duplicate lifecycle service: {name}"),
            Self::NoServices => write!(f, "lifecycle requires at least one service"),
            Self::InvalidGracePeriod => write!(f, "shutdown grace period is too large"),
        }
    }
}

impl Error for ConfigurationError {}

/// Where an error occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// A named registered service.
    Service(String),
    /// The caller-provided shutdown trigger.
    Trigger,
    /// Application cleanup after all services finished.
    Cleanup,
}

/// An observed failure with its original error source.
#[derive(Debug)]
pub struct Failure {
    /// Component that failed.
    pub phase: Phase,
    /// The original error.
    pub source: BoxError,
}

/// Work that remained when the shared deadline expired.
#[derive(Debug, PartialEq, Eq)]
pub enum Unfinished {
    /// These services did not finish; final cleanup was not started.
    Services(Vec<String>),
    /// All services finished, but final cleanup did not.
    Cleanup,
}

/// Shutdown outcome, available on both success and failure.
#[derive(Debug)]
pub struct ShutdownReport {
    /// First cause observed by the coordinator.
    pub reason: ShutdownReason,
    /// All service, trigger, and cleanup errors observed while coordinating.
    pub failures: Vec<Failure>,
    /// Set when the shutdown budget expired.
    pub unfinished: Option<Unfinished>,
}

/// A configuration failure or an unsuccessful shutdown with a detailed report.
#[derive(Debug)]
pub enum LifecycleError {
    /// Invalid coordinator configuration.
    Configuration(ConfigurationError),
    /// A service exited unexpectedly, a participant failed, or the deadline expired.
    Shutdown(ShutdownReport),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => error.fmt(f),
            Self::Shutdown(report) => {
                write!(f, "shutdown after {:?}", report.reason)?;
                for failure in &report.failures {
                    write!(f, "; {:?}: {}", failure.phase, failure.source)?;
                }
                if let Some(unfinished) = &report.unfinished {
                    write!(f, "; deadline expired: {unfinished:?}")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for LifecycleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(error) => Some(error),
            Self::Shutdown(report) => report
                .failures
                .first()
                .map(|failure| failure.source.as_ref() as &(dyn Error + 'static)),
        }
    }
}

type ServiceResult = (String, Result<(), BoxError>);

/// Optional coordinator. Registered futures are polled concurrently without
/// spawning tasks; they may borrow application resources. Panics propagate to
/// the caller, and dropping `run` cancels coordination without running cleanup.
pub struct Lifecycle<'a> {
    options: ShutdownOptions,
    shutdown: Shutdown,
    names: BTreeSet<String>,
    services: FuturesUnordered<BoxFuture<'a, ServiceResult>>,
}

impl<'a> Lifecycle<'a> {
    /// Create a coordinator without installing signals or starting work.
    pub fn new(options: ShutdownOptions) -> Self {
        Self {
            options,
            shutdown: Shutdown::new(),
            names: BTreeSet::new(),
            services: FuturesUnordered::new(),
        }
    }

    /// Obtain a handle for requesting or observing shutdown.
    pub fn shutdown(&self) -> Shutdown {
        self.shutdown.clone()
    }

    /// Register a named long-running future. Work starts when `run` is polled.
    pub fn service<F, E>(
        &mut self,
        name: impl Into<String>,
        future: F,
    ) -> Result<(), ConfigurationError>
    where
        F: Future<Output = Result<(), E>> + Send + 'a,
        E: Into<BoxError>,
    {
        let name = name.into();
        if !self.names.insert(name.clone()) {
            return Err(ConfigurationError::DuplicateService(name));
        }
        self.services.push(Box::pin(
            async move { (name, future.await.map_err(Into::into)) },
        ));
        Ok(())
    }

    /// Run until triggered, requested, or a service exits; drain all participants,
    /// then poll `cleanup` using the remainder of the same shutdown budget.
    ///
    /// Cleanup is never polled while services remain unfinished. The trigger is
    /// dropped once shutdown begins. Repeated requests cannot extend the budget.
    /// Use `std::future::pending::<std::io::Result<ShutdownReason>>()` when only
    /// programmatic shutdown is needed. No error is converted to process exit.
    pub async fn run<T, TE, C, CE>(
        mut self,
        trigger: T,
        cleanup: C,
    ) -> Result<ShutdownReport, LifecycleError>
    where
        T: Future<Output = Result<ShutdownReason, TE>>,
        TE: Into<BoxError>,
        C: Future<Output = Result<(), CE>>,
        CE: Into<BoxError>,
    {
        if self.names.is_empty() {
            return Err(LifecycleError::Configuration(
                ConfigurationError::NoServices,
            ));
        }
        if Instant::now()
            .checked_add(self.options.grace_period)
            .is_none()
        {
            return Err(LifecycleError::Configuration(
                ConfigurationError::InvalidGracePeriod,
            ));
        }
        let mut failures = Vec::new();
        let reason = {
            enum Start<T> {
                Requested,
                Trigger(T),
                Service(ServiceResult),
            }
            let mut trigger = pin!(trigger);
            let mut requested = pin!(self.shutdown.requested());
            let start = std::future::poll_fn(|cx| {
                if requested.as_mut().poll(cx).is_ready() {
                    return Poll::Ready(Start::Requested);
                }
                if let Poll::Ready(result) = trigger.as_mut().poll(cx) {
                    return Poll::Ready(Start::Trigger(result));
                }
                if let Poll::Ready(Some(result)) = self.services.poll_next_unpin(cx) {
                    return Poll::Ready(Start::Service(result));
                }
                Poll::Pending
            })
            .await;
            match start {
                Start::Requested => ShutdownReason::Requested,
                Start::Trigger(Ok(reason)) => reason,
                Start::Trigger(Err(source)) => {
                    failures.push(Failure {
                        phase: Phase::Trigger,
                        source: source.into(),
                    });
                    ShutdownReason::TriggerFailed
                }
                Start::Service((name, result)) => {
                    self.names.remove(&name);
                    if let Err(source) = result {
                        failures.push(Failure {
                            phase: Phase::Service(name.clone()),
                            source,
                        });
                    }
                    if self.shutdown.is_requested() {
                        ShutdownReason::Requested
                    } else {
                        ShutdownReason::ServiceExited(name)
                    }
                }
            }
        };

        let deadline = Instant::now()
            .checked_add(self.options.grace_period)
            .ok_or(LifecycleError::Configuration(
                ConfigurationError::InvalidGracePeriod,
            ))?;
        self.shutdown.request();
        let mut report = ShutdownReport {
            reason,
            failures,
            unfinished: None,
        };

        while !self.services.is_empty() {
            if Instant::now() >= deadline {
                report.unfinished = Some(Unfinished::Services(self.names.into_iter().collect()));
                return Err(LifecycleError::Shutdown(report));
            }
            let mut timer = pin!(sleep_until(deadline));
            let completion = std::future::poll_fn(|cx| {
                if timer.as_mut().poll(cx).is_ready() {
                    return Poll::Ready(None);
                }
                self.services.poll_next_unpin(cx)
            })
            .await;
            match completion {
                None => {
                    report.unfinished =
                        Some(Unfinished::Services(self.names.into_iter().collect()));
                    return Err(LifecycleError::Shutdown(report));
                }
                Some((name, result)) => {
                    self.names.remove(&name);
                    if let Err(source) = result {
                        report.failures.push(Failure {
                            phase: Phase::Service(name),
                            source,
                        });
                    }
                }
            }
        }

        if Instant::now() >= deadline {
            report.unfinished = Some(Unfinished::Cleanup);
            return Err(LifecycleError::Shutdown(report));
        }
        let mut timer = pin!(sleep_until(deadline));
        let mut cleanup = pin!(cleanup);
        let result = std::future::poll_fn(|cx| {
            if timer.as_mut().poll(cx).is_ready() {
                return Poll::Ready(None);
            }
            cleanup.as_mut().poll(cx).map(Some)
        })
        .await;
        match result {
            None => report.unfinished = Some(Unfinished::Cleanup),
            Some(Err(source)) => report.failures.push(Failure {
                phase: Phase::Cleanup,
                source: source.into(),
            }),
            Some(Ok(())) => (),
        }

        if report.failures.is_empty()
            && report.unfinished.is_none()
            && !matches!(
                report.reason,
                ShutdownReason::ServiceExited(_) | ShutdownReason::TriggerFailed
            )
        {
            Ok(report)
        } else {
            Err(LifecycleError::Shutdown(report))
        }
    }
}
