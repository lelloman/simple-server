#![cfg(feature = "engine-lifecycle")]
use simple_server::{
    engine_lifecycle as lifecycle, runtime as execution, test as runtime_test, time,
};
#[path = "support/lifecycle_contracts.rs"]
mod contracts;
