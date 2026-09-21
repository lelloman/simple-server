use simple_server::{
    lifecycle::{Lifecycle, ShutdownOptions, ShutdownReason},
    task_scheduling::{Event, FirstRun, Job, JobConfig, Schedule, Scheduler, SchedulerLimits},
};
use std::{collections::BTreeMap, num::NonZeroUsize, time::Duration};
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let one = NonZeroUsize::new(1).unwrap();
    let mut scheduler = Scheduler::<(), std::io::Error>::new(SchedulerLimits {
        max_running: one,
        max_in_flight: one,
        command_capacity: one,
        pools: BTreeMap::new(),
    })?;
    scheduler.register(Job::new(
        "maintenance",
        JobConfig {
            schedules: vec![Schedule::FixedDelay {
                every: Duration::from_secs(60),
                jitter: Duration::ZERO,
                first: FirstRun::Immediately,
            }],
            ..Default::default()
        },
        |_| async { Ok(()) },
    ))?;
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(2),
    });
    let shutdown = lifecycle.shutdown();
    let stop_after_demonstration = shutdown.clone();
    lifecycle.service(
        "scheduler",
        scheduler.run(shutdown, move |event| {
            if let Event::Completed { completion, .. } = event {
                println!("{}: {:?}", completion.task.name, completion.exit);
                stop_after_demonstration.request();
            }
        }),
    )?;
    lifecycle
        .run(
            std::future::pending::<Result<ShutdownReason, std::io::Error>>(),
            async { Ok::<_, std::io::Error>(()) },
        )
        .await?;
    assert!(scheduler.unfinished().is_empty());
    Ok(())
}
