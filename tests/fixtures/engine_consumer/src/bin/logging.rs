use simple_server::engine_logging::{self, FilterMode, LoggingOptions};
fn main() {
    engine_logging::try_init(LoggingOptions::new("info"), FilterMode::Strict).unwrap();
    engine_logging::init_log_bridge().unwrap();
    let span = tracing::info_span!("standalone", tenant = 7);
    let _entered = span.enter();
    tracing::info!("native logging consumer");
}
