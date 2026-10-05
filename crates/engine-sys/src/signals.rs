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
