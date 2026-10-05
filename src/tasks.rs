//! Explicit ownership of external work and spawned jobs.
//!
//! Cancellation is cooperative. A deadline bounds waiting, not execution. Dropping
//! a task set requests shutdown and detaches unfinished work; only draining proves
//! completion. Neither task handles nor HTTP framework types escape this module.

mod execution {
    pub use tokio::{
        runtime::Handle,
        task::{AbortHandle, Id, JoinSet},
    };
}
mod clock {
    pub use std::time::Instant;
    pub fn now() -> Instant {
        tokio::time::Instant::now().into_std()
    }
    pub fn sleep_until(deadline: Instant) -> tokio::time::Sleep {
        tokio::time::sleep_until(deadline.into())
    }
}
#[path = "tasks/core.rs"]
mod core;
pub use core::*;
