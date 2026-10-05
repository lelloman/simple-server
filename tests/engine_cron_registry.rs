#![cfg(feature = "engine-scheduling")]
use simple_server::{engine_scheduling as scheduling, test as runtime_test, time as clock};
use simple_server::{engine_tasks as tasks, runtime as execution};
#[path = "support/cron_registry_contracts.rs"]
mod contracts;
