#![cfg(feature = "engine-tasks")]
use simple_server::{engine_tasks as tasks, runtime as execution, test as runtime_test};
mod clock {
    pub use simple_server::time::{Instant, advance, sleep, timeout};
    pub fn now() -> Instant {
        Instant::now()
    }
}
#[path = "support/task_contracts.rs"]
mod contracts;
