#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling as scheduling;
use tokio::{test as runtime_test, time as clock};
#[path = "support/scheduling_selection_contracts.rs"]
mod contracts;
