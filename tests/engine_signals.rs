#![cfg(all(feature = "engine-lifecycle", unix))]
use simple_server::{engine_lifecycle::Signals, test as runtime_test};
#[path = "support/signal_contracts.rs"]
mod contracts;
