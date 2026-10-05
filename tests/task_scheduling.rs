#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling as scheduling;
use simple_server::{lifecycle::Shutdown, tasks};
use tokio::{test as runtime_test, time as clock};
#[path = "support/task_scheduling_contracts.rs"]
mod contracts;
