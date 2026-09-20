//! Run with `cargo run --no-default-features --features logging --example logging`.

use simple_server::logging::{LogFormat, LoggingOptions, try_init};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = LoggingOptions::new("info");
    options.format = LogFormat::Json;
    try_init(options)?;
    let startup = tracing::info_span!("startup", service = "example");
    let _entered = startup.enter();
    tracing::info!(ready = true, "logging initialized without a runtime");
    Ok(())
}
