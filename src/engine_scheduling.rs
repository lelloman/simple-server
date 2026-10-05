//! Application-owned scheduling through the shared engine.
//!
//! Enable `engine-scheduling` without default features. Jobs, blocking work and
//! timers execute in the engine. The wrapper retains Tokio's runtime-independent
//! channels, semaphores and selection macros, without its executor or I/O drivers.
//! Policies, retry state, reporting and persistence remain application-owned.
#[cfg(test)]
use crate::test as runtime_test;
use crate::{engine_tasks as tasks, runtime as execution, time as clock};
#[allow(clippy::duplicate_mod)]
#[path = "task_scheduling/mod.rs"]
mod core;
pub use crate::shutdown::Shutdown;
pub use core::*;

#[cfg(feature = "task-policy-core")]
fn task_instant(value: clock::Instant) -> clock::Instant {
    value
}
