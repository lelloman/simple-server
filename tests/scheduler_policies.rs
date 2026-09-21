#![cfg(all(feature = "task-scheduling", feature = "task-policies"))]
use simple_server::{lifecycle::Shutdown, task_policies::*, task_scheduling::*, tasks::TaskExit};
use std::{
    collections::BTreeMap,
    num::{NonZeroU32, NonZeroUsize},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
fn n(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn limits() -> SchedulerLimits {
    SchedulerLimits {
        max_running: n(1),
        max_in_flight: n(4),
        command_capacity: n(8),
        pools: BTreeMap::from([("cpu".into(), n(1))]),
    }
}
fn retries() -> RetryPolicy {
    RetryPolicy {
        max_attempts: NonZeroU32::new(3).unwrap(),
        initial_delay: Duration::from_secs(5),
        max_delay: Duration::from_secs(10),
        jitter: Duration::ZERO,
    }
}
fn configured() -> ExecutionPolicy<&'static str> {
    ExecutionPolicy::default().with_retry(retries(), |_| true)
}
fn immediate() -> JobConfig {
    JobConfig {
        schedules: vec![Schedule::FixedDelay {
            every: Duration::from_secs(60),
            jitter: Duration::ZERO,
            first: FirstRun::Immediately,
        }],
        ..Default::default()
    }
}
#[tokio::test(start_paused = true)]
async fn backoff_releases_global_capacity_but_keeps_job_overlap_reservation() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    s.register(
        Job::new("retry", Default::default(), |ctx| async move {
            if ctx.attempt < 3 {
                Err("transient")
            } else {
                Ok(())
            }
        })
        .with_policy(configured()),
    )
    .unwrap();
    s.register(Job::new("other", Default::default(), |_| async { Ok(()) }))
        .unwrap();
    let (send, mut events) = tokio::sync::mpsc::unbounded_channel();
    let mut attempts = Vec::new();
    let mut other_done = false;
    let driver = async {
        h.trigger("retry", None).await.unwrap().result.unwrap();
        events.recv().await.unwrap();
        assert_eq!(
            h.trigger("retry", None).await.unwrap().result,
            Err(Rejection::Busy)
        );
        h.trigger("other", None).await.unwrap().result.unwrap();
    };
    let origin = tokio::time::Instant::now();
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| match event {
            Event::RetryScheduled { .. } => {
                let _ = send.send(());
            }
            Event::Started {
                job_id, attempt, ..
            } if job_id == "retry" =>
                attempts.push((attempt, tokio::time::Instant::now() - origin)),
            Event::Completed {
                job_id,
                attempt,
                will_retry,
                ..
            } => {
                if job_id == "other" {
                    other_done = true;
                }
                if job_id == "retry" && attempt == 3 {
                    assert!(!will_retry);
                    assert!(other_done);
                    stop.request();
                }
            }
            _ => {}
        }),
        driver
    );
    result.unwrap();
    assert_eq!(
        attempts,
        [
            (1, Duration::ZERO),
            (2, Duration::from_secs(5)),
            (3, Duration::from_secs(15))
        ]
    );
}
#[tokio::test(start_paused = true)]
async fn queue_timeout_does_not_execute_factory_or_count_as_circuit_failure() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    s.register(Job::new("held", Default::default(), |ctx| async move {
        ctx.task.cancelled().await;
        Ok(())
    }))
    .unwrap();
    let starts = Arc::new(AtomicUsize::new(0));
    let counted = starts.clone();
    let mut policy = configured();
    policy.budget.queue_timeout = Some(Duration::from_secs(2));
    policy.circuit = Some(CircuitPolicy {
        failure_threshold: NonZeroU32::new(1).unwrap(),
        cooldown: Duration::from_secs(30),
    });
    s.register(
        Job::new(
            "queued",
            JobConfig {
                max_pending: 1,
                ..Default::default()
            },
            move |_| {
                counted.fetch_add(1, Ordering::SeqCst);
                async { Ok(()) }
            },
        )
        .with_policy(policy),
    )
    .unwrap();
    let mut expired = 0;
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if matches!(event, Event::QueueExpired { .. }) {
                expired += 1;
                stop.request();
            }
        }),
        async {
            h.trigger("held", None).await.unwrap().result.unwrap();
            h.trigger("queued", None).await.unwrap().result.unwrap();
        }
    );
    result.unwrap();
    assert_eq!(expired, 1);
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert_eq!(s.snapshot().circuits["queued"], CircuitSnapshot::default());
}
#[tokio::test]
async fn blocking_runtime_timeout_keeps_capacity_and_preserves_eventual_error() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    let (release, wait) = std::sync::mpsc::channel();
    let wait = Arc::new(Mutex::new(wait));
    let mut policy = configured();
    policy.budget.max_runtime = Some(Duration::from_millis(10));
    s.register(
        Job::blocking("blocking", Default::default(), move |ctx| {
            wait.lock().unwrap().recv().unwrap();
            assert!(ctx.task.is_cancelled());
            Err("eventual source")
        })
        .with_policy(policy),
    )
    .unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let counted = started.clone();
    s.register(Job::new(
        "other",
        JobConfig {
            max_pending: 1,
            ..Default::default()
        },
        move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
            async { Ok(()) }
        },
    ))
    .unwrap();
    let (send, mut timed_out) = tokio::sync::mpsc::unbounded_channel();
    let mut observed = false;
    let driver = async {
        h.trigger("blocking", None).await.unwrap().result.unwrap();
        timed_out.recv().await.unwrap();
        h.trigger("other", None).await.unwrap().result.unwrap();
        h.snapshot().await.unwrap();
        assert_eq!(started.load(Ordering::SeqCst), 0);
        assert_eq!(
            h.trigger("blocking", None).await.unwrap().result,
            Err(Rejection::Busy)
        );
        release.send(()).unwrap();
    };
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| match event {
            Event::RuntimeExceeded { .. } => {
                send.send(()).unwrap();
            }
            Event::Completed {
                job_id,
                completion,
                runtime_exceeded,
                will_retry,
                ..
            } if job_id == "blocking" => {
                assert!(runtime_exceeded);
                assert!(!will_retry);
                assert!(matches!(
                    completion.exit,
                    TaskExit::Finished(Err("eventual source"))
                ));
                observed = true;
            }
            Event::Completed { job_id, .. } if job_id == "other" => stop.request(),
            _ => {}
        }),
        driver
    );
    result.unwrap();
    assert!(observed);
    assert_eq!(started.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn late_completion_observation_does_not_manufacture_runtime_overrun() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let mut policy = ExecutionPolicy::default();
    policy.budget.max_runtime = Some(Duration::from_millis(20));
    s.register(Job::blocking("quick", immediate(), |_| Ok(())).with_policy(policy))
        .unwrap();
    let mut observed = false;
    s.run(stop.clone(), |event| match event {
        // Deliberately stall this test observer to verify actual timestamps.
        Event::Started { .. } => std::thread::sleep(Duration::from_millis(40)),
        Event::RuntimeExceeded { .. } => panic!("quick task was already finished"),
        Event::Completed {
            runtime_exceeded, ..
        } => {
            assert!(!runtime_exceeded);
            observed = true;
            stop.request();
        }
        _ => {}
    })
    .await
    .unwrap();
    assert!(observed);
}
#[tokio::test(start_paused = true)]
async fn shutdown_cancels_pending_retry_without_starting_another_attempt() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let stop = Shutdown::new();
    s.register(Job::new("job", immediate(), |_| async { Err("retry") }).with_policy(configured()))
        .unwrap();
    let mut starts = 0;
    let mut cancelled = 0;
    s.run(stop.clone(), |event| match event {
        Event::Started { .. } => starts += 1,
        Event::RetryScheduled { .. } => stop.request(),
        Event::Cancelled { .. } => cancelled += 1,
        _ => {}
    })
    .await
    .unwrap();
    assert_eq!(starts, 1);
    assert_eq!(cancelled, 1);
}
#[tokio::test]
async fn panic_trips_circuit_without_retry_and_manual_commands_respect_it() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let h = s.handle();
    let mut policy = configured();
    policy.circuit = Some(CircuitPolicy {
        failure_threshold: NonZeroU32::new(1).unwrap(),
        cooldown: Duration::from_secs(30),
    });
    s.register(
        Job::new("panic", Default::default(), |_| {
            panic!("boom");
            #[allow(unreachable_code)]
            async {
                Ok(())
            }
        })
        .with_policy(policy),
    )
    .unwrap();
    let (send, mut observed) = tokio::sync::mpsc::unbounded_channel();
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if let Event::Completed { will_retry, .. } = event {
                assert!(!will_retry);
                send.send(()).unwrap();
            }
        }),
        async {
            h.trigger("panic", None).await.unwrap().result.unwrap();
            observed.recv().await.unwrap();
            assert_eq!(
                h.trigger("panic", None).await.unwrap().result,
                Err(Rejection::CircuitOpen)
            );
            stop.request();
        }
    );
    result.unwrap();
    assert_eq!(s.snapshot().circuits["panic"].consecutive_failures, 1);
}
#[tokio::test]
async fn explicit_cancellation_is_neither_retried_nor_a_circuit_failure() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let h = s.handle();
    let mut policy = configured();
    policy.circuit = Some(CircuitPolicy {
        failure_threshold: NonZeroU32::new(1).unwrap(),
        cooldown: Duration::from_secs(30),
    });
    s.register(
        Job::new("cancel", Default::default(), |ctx| async move {
            ctx.task.cancelled().await;
            Err("cancelled work")
        })
        .with_policy(policy),
    )
    .unwrap();
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if let Event::Completed { will_retry, .. } = event {
                assert!(!will_retry);
                stop.request();
            }
        }),
        async {
            let id = h.trigger("cancel", None).await.unwrap().result.unwrap();
            assert!(h.cancel(id).await.unwrap());
        }
    );
    result.unwrap();
    assert_eq!(s.snapshot().circuits["cancel"], CircuitSnapshot::default());
}
#[tokio::test]
async fn snapshots_restore_controls_without_replaying_active_or_queued_jobs() {
    let config = JobConfig {
        max_pending: 1,
        ..immediate()
    };
    let mut first = Scheduler::<(), &str>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let h = first.handle();
    first
        .register(Job::new("job", config.clone(), |ctx| async move {
            ctx.task.cancelled().await;
            Ok(())
        }))
        .unwrap();
    let mut saved = None;
    let (result, _) = tokio::join!(first.run(stop.clone(), |_| {}), async {
        h.trigger("job", None).await.unwrap().result.unwrap();
        h.set_pause(PauseScope::Global, true, false)
            .await
            .unwrap()
            .unwrap();
        saved = Some(h.snapshot().await.unwrap());
        stop.request();
    });
    result.unwrap();
    let snapshot = saved.unwrap();
    assert!(snapshot.pause.global);
    assert_eq!(snapshot.schedules["job"], [None]);
    let mut restored = Scheduler::<(), &str>::new(limits()).unwrap();
    let stop = Shutdown::new();
    let h = restored.handle();
    let starts = Arc::new(AtomicUsize::new(0));
    let counted = starts.clone();
    restored
        .register(Job::new("job", config, move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
            async { Ok(()) }
        }))
        .unwrap();
    restored.restore(snapshot).unwrap();
    let (result, _) = tokio::join!(
        restored.run(stop.clone(), |event| {
            if matches!(event, Event::Completed { .. }) {
                stop.request();
            }
        }),
        async {
            assert_eq!(
                h.trigger("job", None).await.unwrap().result,
                Err(Rejection::Paused)
            );
            h.set_pause(PauseScope::Global, false, false)
                .await
                .unwrap()
                .unwrap();
            let snapshot = h.snapshot().await.unwrap();
            assert!(snapshot.schedules["job"][0].unwrap() > std::time::SystemTime::now());
            assert_eq!(starts.load(Ordering::SeqCst), 0);
            h.trigger("job", None).await.unwrap().result.unwrap();
        }
    );
    result.unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn pause_cancels_only_matching_active_work_and_rejects_unknown_scopes() {
    let mut cap = limits();
    cap.max_running = n(2);
    let mut s = Scheduler::<(), &str>::new(cap).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    for (name, pool) in [("cpu", Some("cpu".into())), ("other", None)] {
        s.register(Job::new(
            name,
            JobConfig {
                resource_pool: pool,
                ..Default::default()
            },
            |ctx| async move {
                ctx.task.cancelled().await;
                Ok(())
            },
        ))
        .unwrap();
    }
    let (send, mut events) = tokio::sync::mpsc::unbounded_channel();
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| {
            if let Event::Completed { job_id, .. } = event {
                send.send(job_id).unwrap();
            }
        }),
        async {
            h.trigger("cpu", None).await.unwrap().result.unwrap();
            h.trigger("other", None).await.unwrap().result.unwrap();
            assert!(
                h.set_pause(PauseScope::Job("missing".into()), true, false)
                    .await
                    .unwrap()
                    .is_err()
            );
            h.set_pause(PauseScope::Pool("cpu".into()), true, true)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(events.recv().await.unwrap(), "cpu");
            assert!(events.try_recv().is_err());
            assert_eq!(
                h.trigger("cpu", None).await.unwrap().result,
                Err(Rejection::Paused)
            );
            stop.request();
        }
    );
    result.unwrap();
}

#[test]
fn blocking_executor_queue_delay_is_not_execution_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (release, held) = std::sync::mpsc::channel();
        let (send, ready) = tokio::sync::oneshot::channel();
        let busy = tokio::task::spawn_blocking(move || {
            send.send(()).unwrap();
            held.recv().unwrap();
        });
        ready.await.unwrap();
        let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
        let stop = Shutdown::new();
        let mut policy = ExecutionPolicy::default();
        policy.budget.max_runtime = Some(Duration::from_millis(5));
        s.register(Job::blocking("queued-in-tokio", immediate(), |_| Ok(())).with_policy(policy))
            .unwrap();
        let (send, mut admitted) = tokio::sync::mpsc::unbounded_channel();
        let (result, _) = tokio::join!(
            s.run(stop.clone(), |event| match event {
                Event::Admitted { .. } => {
                    send.send(()).unwrap();
                }
                Event::RuntimeExceeded { .. } => panic!("executor queue time is not runtime"),
                Event::Completed {
                    runtime_exceeded, ..
                } => {
                    assert!(!runtime_exceeded);
                    stop.request();
                }
                _ => {}
            }),
            async {
                admitted.recv().await.unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
                release.send(()).unwrap();
            }
        );
        result.unwrap();
        busy.await.unwrap();
    });
}
#[test]
fn snapshot_validation_is_atomic_and_freezes_registration() {
    let make = || {
        let mut scheduler = Scheduler::<(), ()>::new(limits()).unwrap();
        scheduler
            .register(Job::new("job", immediate(), |_| async { Ok(()) }))
            .unwrap();
        scheduler
    };
    let mut original = make();
    let mut saved = original.snapshot();
    saved.pause.global = true;
    original.restore(saved.clone()).unwrap();
    let mut invalid = saved.clone();
    invalid.pause.pools.insert("missing".into());
    assert!(original.restore(invalid).is_err());
    assert_eq!(original.snapshot(), saved);
    assert!(
        original
            .register(Job::new("extra", Default::default(), |_| async { Ok(()) }))
            .is_err()
    );
}

#[tokio::test(start_paused = true)]
async fn retry_reservation_does_not_consume_a_pending_slot_twice() {
    let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
    let h = s.handle();
    let stop = Shutdown::new();
    s.register(
        Job::new(
            "job",
            JobConfig {
                max_pending: 1,
                ..Default::default()
            },
            |ctx| async move {
                if ctx.attempt == 1 {
                    Err("retry")
                } else {
                    Ok(())
                }
            },
        )
        .with_policy(configured()),
    )
    .unwrap();
    let (send, mut events) = tokio::sync::mpsc::unbounded_channel();
    let mut final_runs = 0;
    let (result, _) = tokio::join!(
        s.run(stop.clone(), |event| match event {
            Event::RetryScheduled { .. } => {
                let _ = send.send(());
            }
            Event::Completed {
                will_retry: false, ..
            } => {
                final_runs += 1;
                if final_runs == 2 {
                    stop.request();
                }
            }
            _ => {}
        }),
        async {
            h.trigger("job", None).await.unwrap().result.unwrap();
            events.recv().await.unwrap();
            h.trigger("job", None).await.unwrap().result.unwrap();
            assert_eq!(
                h.trigger("job", None).await.unwrap().result,
                Err(Rejection::Busy)
            );
        }
    );
    result.unwrap();
    assert_eq!(final_runs, 2);
}
