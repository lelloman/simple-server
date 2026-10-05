//! Engine-backed shutdown coordination and explicit operating-system signals.
//!
//! Enable `engine-lifecycle` without default features to avoid compiling Tokio.
//! The application owns the runtime and chooses when to install signal handlers.
//! The same coordination algorithm and runtime-independent shutdown token are
//! used by `lifecycle`, whose clock and signals remain on the source runtime.
use crate::time as clock;
// Each parent intentionally supplies its own clock to the shared algorithm.
#[allow(clippy::duplicate_mod)]
#[path = "lifecycle/core.rs"]
mod core;
pub use core::*;

/// Explicit engine signal registration. Dropping this value does not promise
/// to restore the operating system's default signal behavior.
pub struct Signals(simple_server_sys::Signals);
impl Signals {
    pub fn install() -> std::io::Result<Self> {
        simple_server_sys::Signals::install().map(Self)
    }
    pub async fn wait(self) -> std::io::Result<ShutdownReason> {
        let signal = match self.0.wait().await? {
            1 => Signal::Interrupt,
            2 => Signal::Terminate,
            _ => unreachable!("sys bindings validate engine signals"),
        };
        Ok(ShutdownReason::Signal(signal))
    }
}
