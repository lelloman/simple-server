use simple_server::{
    engine_lifecycle::{Lifecycle, ShutdownOptions, ShutdownReason, Signals},
    time::{Instant, sleep},
};
use std::{io, time::Duration};

#[simple_server::main(flavor = "current_thread", start_paused = true)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Registration is explicit and confined to this disposable fixture process.
    drop(Signals::install()?);
    let mut completed = false;
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(5),
    });
    let shutdown = lifecycle.shutdown();
    let completed_ref = &mut completed;
    lifecycle.service("worker", async move {
        shutdown.requested().await;
        sleep(Duration::from_secs(2)).await;
        *completed_ref = true;
        Ok::<_, io::Error>(())
    })?;
    let start = Instant::now();
    let report = lifecycle
        .run(
            async { Ok::<_, io::Error>(ShutdownReason::Triggered) },
            async {
                sleep(Duration::from_secs(1)).await;
                Ok::<_, io::Error>(())
            },
        )
        .await?;
    assert_eq!(report.reason, ShutdownReason::Triggered);
    assert!(completed);
    assert_eq!(start.elapsed(), Duration::from_secs(3));
    println!("engine lifecycle consumer passed");
    Ok(())
}
