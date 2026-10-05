//! Application-owned scheduling on the source runtime.
use crate::tasks;
#[cfg(test)]
use tokio::test as runtime_test;
use tokio::{task as execution, time as clock};
#[allow(clippy::duplicate_mod)]
#[path = "mod.rs"]
mod core;
pub use core::*;

#[cfg(feature = "task-policy-core")]
pub(super) fn task_instant(value: std::time::Instant) -> clock::Instant {
    value.into()
}
