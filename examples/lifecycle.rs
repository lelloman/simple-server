use std::{io, time::Duration};

use simple_server::{
    axum::{Router, routing::get},
    http,
    lifecycle::{Lifecycle, ShutdownOptions, Signals},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signals = Signals::install()?;
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(30),
    });
    let listener = http::bind("127.0.0.1:3000").await?;
    println!("Listening on {}", listener.local_addr()?);
    let app = Router::new().route("/", get(|| async { "hello" }));
    lifecycle.service("http", http::serve(listener, app, lifecycle.shutdown()))?;
    let report = lifecycle
        .run(signals.wait(), async { Ok::<_, io::Error>(()) })
        .await?;
    println!("Stopped: {:?}", report.reason);
    Ok(())
}
