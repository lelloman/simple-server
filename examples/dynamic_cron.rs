use simple_server::{
    task_scheduling::{CronEntryConfig, CronRegistry, CronSchedule},
    tasks::WorkTracker,
};
use std::time::SystemTime;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = CronRegistry::new();
    let mut config = CronEntryConfig::new(CronSchedule::parse("* * * * * *")?);
    config.enabled = false;
    registry.register("maintenance", config, SystemTime::now())?;
    let work = WorkTracker::default();

    // The application chooses its own control protocol and bounded transport.
    let (admin, mut commands) = tokio::sync::mpsc::channel(8);
    admin.send(("maintenance".to_string(), true)).await?;
    drop(admin);
    loop {
        tokio::select! {
            biased; // This application chooses control priority over a due tick.
            Some((id, enabled)) = commands.recv() => {
                registry.set_enabled(&id, enabled, SystemTime::now())?;
            }
            occurrence = registry.next_due() => {
                let Some(occurrence) = occurrence else { break; };
                let guard = work.try_acquire(occurrence.id.clone())?;
                tokio::spawn(async move {
                    let _guard = guard;
                    println!("{} due at {:?}", occurrence.id, occurrence.scheduled_for);
                    // Application-owned execution, storage and outcome reporting go here.
                });
                break; // End this demonstration after one occurrence.
            }
        }
    }
    registry.close();
    work.close();
    work.wait().await;
    Ok(())
}
