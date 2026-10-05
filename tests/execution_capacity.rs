#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling as scheduling;
use tokio as execution;
use tokio::{test as runtime_test, time as clock};
#[path = "support/execution_capacity_contracts.rs"]
mod contracts;
