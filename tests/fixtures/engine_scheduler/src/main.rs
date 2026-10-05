use simple_server::{
    engine_scheduling::{
        Event, FirstRun, Job, JobConfig, Schedule, Scheduler, SchedulerLimits, Shutdown,
    },
    engine_tasks::TaskExit,
    task_policies::{ExecutionPolicy, RetryPolicy},
    time::{Instant, sleep},
};
use std::{
    collections::BTreeMap,
    num::{NonZeroU32, NonZeroUsize},
    time::Duration,
};

#[simple_server::main(flavor = "current_thread", start_paused = true)]
async fn main() {
    let mut scheduler = Scheduler::<(), &'static str>::with_seed(
        SchedulerLimits {
            max_running: NonZeroUsize::new(1).unwrap(),
            max_in_flight: NonZeroUsize::new(2).unwrap(),
            command_capacity: NonZeroUsize::new(2).unwrap(),
            pools: BTreeMap::new(),
        },
        1,
    )
    .unwrap();
    scheduler
        .register(
            Job::new(
                "retry",
                JobConfig {
                    schedules: vec![Schedule::FixedDelay {
                        every: Duration::from_secs(60),
                        jitter: Duration::ZERO,
                        first: FirstRun::Immediately,
                    }],
                    ..Default::default()
                },
                |ctx| async move {
                    sleep(Duration::from_secs(2)).await;
                    if ctx.attempt == 1 {
                        Err("retry me")
                    } else {
                        Ok(())
                    }
                },
            )
            .with_policy(ExecutionPolicy::default().with_retry(
                RetryPolicy {
                    max_attempts: NonZeroU32::new(2).unwrap(),
                    initial_delay: Duration::from_secs(3),
                    max_delay: Duration::from_secs(3),
                    jitter: Duration::ZERO,
                },
                |_| true,
            )),
        )
        .unwrap();
    let start = Instant::now();
    let shutdown = Shutdown::new();
    let mut completed = Vec::new();
    scheduler
        .run(shutdown.clone(), |event| {
            if let Event::Completed {
                attempt,
                completion,
                ..
            } = event
            {
                completed.push(attempt);
                if matches!(completion.exit, TaskExit::Finished(Ok(()))) {
                    shutdown.request();
                }
            }
        })
        .await
        .unwrap();
    assert_eq!(completed, [1, 2]);
    assert_eq!(start.elapsed(), Duration::from_secs(7));
    assert!(scheduler.unfinished().is_empty());
    println!("engine scheduler consumer passed");
}
