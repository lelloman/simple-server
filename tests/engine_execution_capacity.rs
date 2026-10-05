#![cfg(feature = "engine-scheduling")]
use simple_server::runtime as execution;
use simple_server::{engine_scheduling as scheduling, test as runtime_test, time as clock};
#[path = "support/execution_capacity_contracts.rs"]
mod contracts;
