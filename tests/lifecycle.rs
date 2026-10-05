#![cfg(feature = "lifecycle")]
use simple_server::lifecycle;
use tokio::{task as execution, test as runtime_test, time};
#[path = "support/lifecycle_contracts.rs"]
mod contracts;
