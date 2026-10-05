#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling as scheduling;
use simple_server::tasks;
use tokio as execution;
use tokio::{test as runtime_test, time as clock};
#[path = "support/cron_registry_contracts.rs"]
mod contracts;
