//! Bounded task ownership and external work tracking through the shared engine.
//!
//! Enable `engine-tasks` without default features to avoid compiling Tokio.
//! Jobs use the application's engine runtime; deadlines and task timings use
//! `crate::time::Instant`. Dropping a set requests cooperative shutdown and
//! detaches unfinished work. Only explicitly draining proves completion.
use crate::runtime as execution;
mod clock {
    pub use crate::time::{Instant, sleep_until};
    pub fn now() -> Instant {
        Instant::now()
    }
}
// Each parent supplies its own runtime and clock to the same ownership algorithm.
#[allow(clippy::duplicate_mod)]
#[path = "tasks/core.rs"]
mod core;
pub use core::*;
