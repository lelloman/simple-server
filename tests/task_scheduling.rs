#![cfg(feature = "task-scheduling")]
use simple_server::{
    lifecycle::Shutdown,
    task_scheduling::*,
    tasks::{ShutdownBehavior, TaskExit},
};
use std::{
    collections::BTreeMap,
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, SystemTime},
};
fn n(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn limits() -> SchedulerLimits {
    SchedulerLimits {
        max_running: n(2),
        max_in_flight: n(4),
        command_capacity: n(4),
        pools: BTreeMap::from([("cpu".into(), n(1)), ("io".into(), n(1))]),
    }
}
fn delay(seconds: u64) -> Schedule {
    Schedule::FixedDelay {
        every: Duration::from_secs(seconds),
        jitter: Duration::ZERO,
        first: FirstRun::Immediately,
    }
}

#[test]
fn cron_seconds_validation_and_utc_boundary() {
    assert!(CronSchedule::parse("* * * * *").is_err());
    let cron = CronSchedule::parse("0 0 0 1 1 *").unwrap();
    // 2024-01-01T00:00:00Z: next yearly occurrence is 2025, a leap year apart.
    let january = SystemTime::UNIX_EPOCH + Duration::from_secs(1_704_067_200);
    assert_eq!(
        cron.next_after(january)
            .unwrap()
            .duration_since(january)
            .unwrap(),
        Duration::from_secs(366 * 86400)
    );
    assert!(
        CronSchedule::parse("0 0 0 1 1 * 2020")
            .unwrap()
            .next_after(january)
            .is_none()
    );
}
#[test]
fn cron_future_year_resets_lower_calendar_fields() {
    let after: SystemTime = chrono::DateTime::parse_from_rfc3339("2024-02-28T23:59:59Z")
        .unwrap()
        .into();
    let expected: SystemTime = chrono::DateTime::parse_from_rfc3339("2026-01-01T03:15:00Z")
        .unwrap()
        .into();
    let schedule = CronSchedule::parse("0 15 3 1,15 * * 2026-2030").unwrap();
    assert_eq!(schedule.next_after(after), Some(expected));
}

#[test]
fn pure_interval_timing_and_jitter_are_deterministic() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let rate = Schedule::FixedRate {
        every: Duration::from_secs(10),
        first: FirstRun::AfterInterval,
    };
    assert_eq!(
        rate.first_after(now, 0.0),
        Some(now + Duration::from_secs(10))
    );
    assert_eq!(
        rate.after_tick(now, now + Duration::from_secs(95)),
        Some(now + Duration::from_secs(100))
    );
    let delay = Schedule::FixedDelay {
        every: Duration::from_secs(10),
        jitter: Duration::from_secs(4),
        first: FirstRun::Immediately,
    };
    assert_eq!(delay.first_after(now, 0.5), Some(now));
    assert_eq!(
        delay.after_completion(now, 0.5),
        Some(now + Duration::from_secs(12))
    );
    assert!(
        Schedule::FixedRate {
            every: Duration::ZERO,
            first: Default::default()
        }
        .validate()
        .is_err()
    );
}
#[tokio::test]
async fn setup_rejects_duplicates_unknown_pools_and_invalid_limits() {
    let mut invalid = limits();
    invalid.max_in_flight = n(1);
    assert!(Scheduler::<(), ()>::new(invalid).is_err());
    let mut s = Scheduler::<(), ()>::new(limits()).unwrap();
    s.register(Job::new("job", Default::default(), |_| async { Ok(()) }))
        .unwrap();
    assert!(
        s.register(Job::new("job", Default::default(), |_| async { Ok(()) }))
            .is_err()
    );
    assert!(
        s.register(Job::new(
            "other",
            JobConfig {
                resource_pool: Some("missing".into()),
                ..Default::default()
            },
            |_| async { Ok(()) }
        ))
        .is_err()
    );
}
#[tokio::test]
async fn saturated_pool_does_not_block_another_pool_and_queues_are_bounded() {
    let mut s = Scheduler::<(), ()>::new(limits()).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    let cpu = Arc::new(tokio::sync::Semaphore::new(0));
    let io = Arc::new(tokio::sync::Semaphore::new(0));
    for (id, gate) in [("cpu", cpu.clone()), ("io", io.clone())] {
        s.register(Job::new(
            id,
            JobConfig {
                resource_pool: Some(id.into()),
                max_pending: 1,
                ..Default::default()
            },
            move |_| {
                let gate = gate.clone();
                async move {
                    let _permit = gate.acquire().await.unwrap();
                    Ok(())
                }
            },
        ))
        .unwrap();
    }
    let mut started = Vec::new();
    let producer = async {
        assert!(h.trigger("cpu", None).await.unwrap().result.is_ok());
        assert!(h.trigger("cpu", None).await.unwrap().result.is_ok());
        assert_eq!(
            h.trigger("cpu", None).await.unwrap().result,
            Err(Rejection::Busy)
        );
        assert!(h.trigger("io", None).await.unwrap().result.is_ok());
        cpu.add_permits(2);
        io.add_permits(1);
        stop.request();
    };
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if let Event::Started { job_id, .. } = event {
                started.push(job_id);
            }
        }),
        producer
    );
    result.unwrap();
    assert_eq!(started, ["cpu", "io"]);
}
#[tokio::test]
async fn typed_event_fanout_and_failure_observation() {
    let mut s = Scheduler::<String, &'static str>::new(limits()).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    s.register(Job::new(
        "failure",
        JobConfig {
            events: vec!["changed".into()],
            ..Default::default()
        },
        |ctx: JobContext<String>| async move {
            assert_eq!(ctx.parameters.as_deref().unwrap(), "payload");
            Err("original")
        },
    ))
    .unwrap();
    s.register(Job::new(
        "panic",
        JobConfig {
            events: vec!["changed".into()],
            ..Default::default()
        },
        |_| {
            panic!("factory panic");
            #[allow(unreachable_code)]
            async {
                Ok(())
            }
        },
    ))
    .unwrap();
    let mut outcomes = Vec::new();
    let mut admitted = 0;
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if let Event::Completed { completion, .. } = event {
                outcomes.push(completion.exit);
                if outcomes.len() == 2 {
                    stop.request();
                }
            }
        }),
        async {
            admitted = h
                .emit("changed", Some("payload".into()))
                .await
                .unwrap()
                .len();
        }
    );
    result.unwrap();
    assert_eq!(admitted, 2);
    assert!(
        outcomes
            .iter()
            .any(|e| matches!(e, TaskExit::Finished(Err("original"))))
    );
    assert!(outcomes.iter().any(|e| matches!(e, TaskExit::Panicked(_))));
    assert_eq!(h.trigger("failure", None).await, Err(Rejection::Closed));
}
#[tokio::test(start_paused = true)]
async fn fixed_delay_is_measured_from_completion_and_manual_runs_do_not_reset_it() {
    let mut s = Scheduler::<(), ()>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let h = s.handle();
    s.register(Job::new(
        "job",
        JobConfig {
            schedules: vec![delay(10)],
            ..Default::default()
        },
        |_| async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            Ok(())
        },
    ))
    .unwrap();
    let (send, mut receive) =
        tokio::sync::mpsc::unbounded_channel::<(Trigger, tokio::time::Instant)>();
    let driver = async {
        assert_eq!(
            receive.recv().await.unwrap(),
            (Trigger::Scheduled(0), tokio::time::Instant::now())
        );
        tokio::time::sleep(Duration::from_secs(4)).await;
        h.trigger("job", None).await.unwrap().result.unwrap();
        assert!(matches!(receive.recv().await.unwrap().0, Trigger::Manual));
        let (_, at) = receive.recv().await.unwrap();
        assert_eq!(at, tokio::time::Instant::now());
        stop.request();
    };
    let origin = tokio::time::Instant::now();
    let mut scheduled = 0;
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if let Event::Admitted { trigger, .. } = event {
                if matches!(trigger, Trigger::Scheduled(_)) {
                    scheduled += 1;
                    if scheduled == 2 {
                        assert_eq!(
                            tokio::time::Instant::now() - origin,
                            Duration::from_secs(13)
                        );
                    }
                }
                send.send((trigger, tokio::time::Instant::now())).unwrap();
            }
        }),
        driver
    );
    result.unwrap();
    assert_eq!(scheduled, 2);
}
#[tokio::test]
async fn outer_deadline_keeps_scheduler_tasks_available_for_later_drain() {
    let mut s = Scheduler::<(), ()>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let worker = gate.clone();
    s.register(Job::new(
        "claim",
        JobConfig {
            schedules: vec![delay(100)],
            shutdown: ShutdownBehavior::FinishOnShutdown,
            ..Default::default()
        },
        move |_| {
            let worker = worker.clone();
            async move {
                let _guard = worker.acquire().await.unwrap();
                Ok(())
            }
        },
    ))
    .unwrap();
    let result = tokio::time::timeout(
        Duration::from_millis(10),
        s.run(stop.clone(), |event| {
            if matches!(event, Event::Started { .. }) {
                stop.request();
            }
        }),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(s.unfinished().len(), 1);
    gate.add_permits(1);
    s.run(stop, |_| {}).await.unwrap();
    assert!(s.unfinished().is_empty());
}

#[tokio::test]
async fn shutdown_requested_by_observer_stops_dispatch_and_further_admission() {
    let mut s = Scheduler::<(), ()>::new(limits()).unwrap();
    let stop = Shutdown::new();
    for name in ["a", "b"] {
        s.register(Job::new(
            name,
            JobConfig {
                schedules: vec![delay(60)],
                ..Default::default()
            },
            |_| {
                panic!("shutdown must prevent dispatch");
                #[allow(unreachable_code)]
                async {
                    Ok(())
                }
            },
        ))
        .unwrap();
    }
    let mut cancelled = 0;
    let mut rejected = 0;
    s.run(stop.clone(), |event| match event {
        Event::Admitted { .. } => stop.request(),
        Event::Cancelled { .. } => cancelled += 1,
        Event::Rejected {
            reason: Rejection::Closed,
            ..
        } => rejected += 1,
        Event::Started { .. } => panic!("started after shutdown"),
        _ => {}
    })
    .await
    .unwrap();
    assert_eq!(cancelled, 1);
    assert_eq!(rejected, 1);
}
