#![cfg(all(feature = "lifecycle", unix))]
use simple_server::lifecycle::Signals;
use tokio::test as runtime_test;
#[path = "support/signal_contracts.rs"]
mod contracts;
