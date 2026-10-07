use crate::{Operation, Resource, current, resource_new, unframe};
use std::io;

/// Explicit signal registration owned by the native engine. Drop unregisters
/// these receivers, but does not restore process-wide default signal handlers.
pub struct Signals(Resource);
impl Signals {
    pub fn install() -> io::Result<Self> {
        current()?;
        let bytes = resource_new(b"{\"op\":\"shutdown_signals\"}")?;
        let (_, id) = unframe(&bytes)?;
        let id = u64::from_le_bytes(
            id.try_into()
                .map_err(|_| io::Error::other("invalid engine signal registration"))?,
        );
        Ok(Self(Resource::new(10, id)))
    }
    /// Consume the registration and return 1 for interrupt or 2 for terminate.
    pub async fn wait(self) -> io::Result<u8> {
        let command = format!("{{\"op\":\"shutdown_signal_wait\",\"id\":{}}}", self.0.id());
        let output = Operation::new(command.as_bytes())?.await?;
        match output.as_slice() {
            [signal @ (1 | 2)] => Ok(*signal),
            _ => Err(io::Error::other("shutdown signal stream closed")),
        }
    }
}

/// A supported Unix signal; the numeric platform signal never crosses the ABI.
#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
pub enum UnixSignalKind {
    Hangup,
    Interrupt,
    Terminate,
}

/// Repeatable Unix signal receiver. Installation requires an engine runtime.
/// Drop releases this receiver, not the process-wide signal handler.
#[cfg(unix)]
pub struct UnixSignal(Resource);
#[cfg(unix)]
impl UnixSignal {
    pub fn install(kind: UnixSignalKind) -> io::Result<Self> {
        current()?;
        let name = match kind {
            UnixSignalKind::Hangup => "hangup",
            UnixSignalKind::Interrupt => "interrupt",
            UnixSignalKind::Terminate => "terminate",
        };
        let bytes =
            resource_new(format!("{{\"op\":\"unix_signal\",\"kind\":\"{name}\"}}").as_bytes())?;
        let (_, id) = unframe(&bytes)?;
        let id = u64::from_le_bytes(
            id.try_into()
                .map_err(|_| io::Error::other("invalid Unix signal registration"))?,
        );
        Ok(Self(Resource::new(23, id)))
    }
    /// Wait for the next notification. Cancellation does not consume future signals.
    pub async fn recv(&mut self) -> io::Result<()> {
        let command = format!("{{\"op\":\"unix_signal_wait\",\"id\":{}}}", self.0.id());
        match Operation::new(command.as_bytes())?.await?.as_slice() {
            [1] => Ok(()),
            _ => Err(io::Error::other("Unix signal stream closed")),
        }
    }
}
