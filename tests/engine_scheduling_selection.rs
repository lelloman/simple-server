#![cfg(feature = "engine-scheduling")]
use simple_server::{engine_scheduling as scheduling, test as runtime_test, time as clock};
#[path = "support/scheduling_selection_contracts.rs"]
mod contracts;
