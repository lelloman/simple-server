use super::execution::{Id, JoinError as BackendJoinError};
use std::{any::Any, error::Error, fmt};

/// Identity of a spawned batch task. Identifiers are comparable and printable;
/// applications must not infer ordering of execution from their values.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BatchTaskId(Id);
impl fmt::Display for BatchTaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An owned batch-task failure, preserving the original panic payload.
#[derive(Debug)]
pub struct JoinError(pub(super) BackendJoinError);
impl JoinError {
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
    pub fn is_panic(&self) -> bool {
        self.0.is_panic()
    }
    pub fn id(&self) -> BatchTaskId {
        BatchTaskId(self.0.id())
    }
    /// Return the original payload. Panics if the task was cancelled.
    pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
        self.0.into_panic()
    }
    pub fn try_into_panic(self) -> Result<Box<dyn Any + Send + 'static>, Self> {
        self.0.try_into_panic().map_err(Self)
    }
}
impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Error for JoinError {}
