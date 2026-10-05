#![cfg(feature = "tasks")]
use simple_server::tasks;
use tokio::{task as execution, test as runtime_test};
mod clock {
    pub use std::time::Instant;
    pub use tokio::time::{advance, sleep, timeout};
    pub fn now() -> Instant {
        tokio::time::Instant::now().into_std()
    }
}
#[path = "support/task_contracts.rs"]
mod contracts;
