//! Cooperative shutdown notification and coordination of application-owned futures.
//!
//! The deadline bounds how long [`Lifecycle::run`] waits, not the lifetime of
//! detached tasks, upgraded connections, or blocking work. No runtime, logging
//! subscriber, or process exit policy is installed by this module.

use std::{collections::BTreeSet, error::Error, fmt, future::Future, io, time::Duration};

use futures_util::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use tokio::{sync::watch, time::Instant};

/// An application or service error, preserving its original source.
pub type BoxError = Box<dyn Error + Send + Sync>;

/// Sticky, idempotent shutdown notification shared by participants.
///
/// Dropping a handle does not request shutdown. Existing cancellation tokens can
/// be bridged by awaiting [`Self::requested`] and cancelling them in application code.
#[derive(Clone, Debug)]
pub struct Shutdown {
    sender: watch::Sender<bool>,
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Shutdown {
    /// Create an independent notification.
    pub fn new() -> Self {
        let (sender, _) = watch::channel(false);
        Self { sender }
    }

    /// Request shutdown for all current and future observers.
    pub fn request(&self) {
        self.sender.send_if_modified(|requested| {
            if *requested {
                false
            } else {
                *requested = true;
                true
            }
        });
    }

    /// Whether shutdown has been requested.
    pub fn is_requested(&self) -> bool {
        *self.sender.borrow()
    }

    /// Wait for shutdown, including a request made before this call.
    pub async fn requested(&self) {
        let mut receiver = self.sender.subscribe();
        // This handle owns a sender, so the channel cannot close while waiting.
        let _ = receiver.wait_for(|requested| *requested).await;
    }
}

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

/// Explicit process-wide signal registration. Dropping this value does not
/// promise to restore the operating system's default signal behavior.
pub struct Signals {
    #[cfg(unix)]
    interrupt: tokio::signal::unix::Signal,
    #[cfg(unix)]
    terminate: tokio::signal::unix::Signal,
    #[cfg(windows)]
    interrupt: tokio::signal::windows::CtrlC,
}

impl Signals {
    /// Install handlers in the current Tokio runtime, reporting registration errors.
    pub fn install() -> io::Result<Self> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            Ok(Self {
                interrupt: signal(SignalKind::interrupt())?,
                terminate: signal(SignalKind::terminate())?,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                interrupt: tokio::signal::windows::ctrl_c()?,
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "OS shutdown signals are unsupported on this platform",
            ))
        }
    }

    /// Consume the registration and wait for the first signal.
    pub async fn wait(mut self) -> io::Result<ShutdownReason> {
        #[cfg(unix)]
        let (received, signal) = tokio::select! {
            received = self.interrupt.recv() => (received, Signal::Interrupt),
            received = self.terminate.recv() => (received, Signal::Terminate),
        };
        #[cfg(windows)]
        let (received, signal) = (self.interrupt.recv().await, Signal::Interrupt);
        #[cfg(any(unix, windows))]
        {
            received.ok_or_else(|| io::Error::other("shutdown signal stream closed"))?;
            Ok(ShutdownReason::Signal(signal))
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "OS shutdown signals are unsupported on this platform",
            ))
        }
    }
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
            tokio::pin!(trigger);
            tokio::select! {
                // An already-requested shutdown must not become an early-exit error.
                biased;
                () = self.shutdown.requested() => ShutdownReason::Requested,
                result = &mut trigger => match result {
                    Ok(reason) => reason,
                    Err(source) => {
                        failures.push(Failure { phase: Phase::Trigger, source: source.into() });
                        ShutdownReason::TriggerFailed
                    }
                },
                Some((name, result)) = self.services.next() => {
                    self.names.remove(&name);
                    if let Err(source) = result {
                        failures.push(Failure { phase: Phase::Service(name.clone()), source });
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
            tokio::select! {
                biased;
                () = tokio::time::sleep_until(deadline) => {
                    report.unfinished = Some(Unfinished::Services(self.names.into_iter().collect()));
                    return Err(LifecycleError::Shutdown(report));
                },
                Some((name, result)) = self.services.next() => {
                    self.names.remove(&name);
                    if let Err(source) = result {
                        report.failures.push(Failure { phase: Phase::Service(name), source });
                    }
                }
            }
        }

        if Instant::now() >= deadline {
            report.unfinished = Some(Unfinished::Cleanup);
            return Err(LifecycleError::Shutdown(report));
        }
        tokio::select! {
            biased;
            () = tokio::time::sleep_until(deadline) => report.unfinished = Some(Unfinished::Cleanup),
            result = cleanup => {
                if let Err(source) = result {
                    report.failures.push(Failure { phase: Phase::Cleanup, source: source.into() });
                }
            }
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
