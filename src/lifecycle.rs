//! Cooperative shutdown notification and coordination of application-owned futures.
//!
//! The deadline bounds how long [`Lifecycle::run`] waits, not the lifetime of
//! detached tasks, upgraded connections, or blocking work. No runtime, logging
//! subscriber, or process exit policy is installed by this module.

use std::io;
use tokio::time as clock;
#[path = "lifecycle/core.rs"]
mod core;
pub use core::*;

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
