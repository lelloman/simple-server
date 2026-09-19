#![cfg(feature = "lifecycle")]

use std::{future::pending, io, sync::Arc, time::Duration};

use simple_server::lifecycle::{
    ConfigurationError, Lifecycle, LifecycleError, Phase, Shutdown, ShutdownOptions,
    ShutdownReason, ShutdownReport, Unfinished,
};
use tokio::{sync::Mutex, time::Instant};

fn coordinator(seconds: u64) -> Lifecycle<'static> {
    Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(seconds),
    })
}

async fn no_trigger() -> io::Result<ShutdownReason> {
    pending().await
}

async fn cleanup() -> io::Result<()> {
    Ok(())
}

fn report(error: LifecycleError) -> ShutdownReport {
    match error {
        LifecycleError::Shutdown(report) => report,
        error => panic!("expected shutdown report: {error}"),
    }
}

#[tokio::test]
async fn notification_is_sticky_shared_and_independent() {
    let shutdown = Shutdown::new();
    let other = Shutdown::new();
    let first = shutdown.clone();
    let second = shutdown.clone();
    let waiters = async { tokio::join!(first.requested(), second.requested()) };
    let request = async {
        tokio::task::yield_now().await;
        shutdown.request();
        shutdown.request();
    };
    tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(waiters, request);
        shutdown.requested().await;
    })
    .await
    .unwrap();
    assert!(shutdown.is_requested());
    drop(other.clone());
    assert!(!other.is_requested());
}

#[tokio::test(start_paused = true)]
async fn services_drain_concurrently_before_cleanup() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut lifecycle = coordinator(10);
    for name in ["api", "metrics", "scheduler"] {
        let shutdown = lifecycle.shutdown();
        let events = events.clone();
        lifecycle
            .service(name, async move {
                shutdown.requested().await;
                tokio::time::sleep(Duration::from_secs(4)).await;
                events.lock().await.push(name);
                Ok::<_, io::Error>(())
            })
            .unwrap();
    }
    let start = Instant::now();
    let result = lifecycle
        .run(
            async { Ok::<_, io::Error>(ShutdownReason::Triggered) },
            async {
                assert_eq!(events.lock().await.len(), 3);
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok::<_, io::Error>(())
            },
        )
        .await
        .unwrap();
    assert_eq!(result.reason, ShutdownReason::Triggered);
    assert_eq!(Instant::now() - start, Duration::from_secs(6));
}

#[tokio::test(start_paused = true)]
async fn programmatic_request_wakes_coordinator_and_supports_borrowing() {
    let mut finished = false;
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(5),
    });
    let shutdown = lifecycle.shutdown();
    let participant = shutdown.clone();
    let finished_ref = &mut finished;
    lifecycle
        .service("borrowed", async move {
            participant.requested().await;
            *finished_ref = true;
            Ok::<_, io::Error>(())
        })
        .unwrap();
    let request = async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        shutdown.request();
    };
    let (result, ()) = tokio::join!(lifecycle.run(no_trigger(), cleanup()), request);
    assert_eq!(result.unwrap().reason, ShutdownReason::Requested);
    assert!(finished);
}

#[tokio::test]
async fn request_before_run_is_normal_shutdown() {
    let mut lifecycle = coordinator(5);
    lifecycle.shutdown().request();
    lifecycle.service("already-done", cleanup()).unwrap();
    assert_eq!(
        lifecycle.run(no_trigger(), cleanup()).await.unwrap().reason,
        ShutdownReason::Requested
    );
}

#[tokio::test]
async fn early_success_is_an_error_but_siblings_still_drain() {
    let mut lifecycle = coordinator(5);
    let shutdown = lifecycle.shutdown();
    lifecycle.service("early", cleanup()).unwrap();
    lifecycle
        .service("sibling", async move {
            shutdown.requested().await;
            Err(io::Error::other("sibling drain failed"))
        })
        .unwrap();
    let outcome = report(lifecycle.run(no_trigger(), cleanup()).await.unwrap_err());
    assert_eq!(
        outcome.reason,
        ShutdownReason::ServiceExited("early".into())
    );
    assert_eq!(outcome.failures.len(), 1);
    assert_eq!(outcome.failures[0].phase, Phase::Service("sibling".into()));
    assert!(outcome.unfinished.is_none());
}

#[tokio::test(start_paused = true)]
async fn timeout_preserves_initial_error_and_skips_cleanup() {
    let mut lifecycle = coordinator(5);
    lifecycle
        .service("failed", async { Err(io::Error::other("original")) })
        .unwrap();
    lifecycle
        .service("stuck", pending::<io::Result<()>>())
        .unwrap();
    let start = Instant::now();
    let outcome = report(
        lifecycle
            .run(no_trigger(), async {
                panic!("must not clean up resources used by unfinished services");
                #[allow(unreachable_code)]
                Ok::<_, io::Error>(())
            })
            .await
            .unwrap_err(),
    );
    assert_eq!(Instant::now() - start, Duration::from_secs(5));
    assert_eq!(outcome.failures[0].source.to_string(), "original");
    assert!(
        outcome.failures[0]
            .source
            .downcast_ref::<io::Error>()
            .is_some()
    );
    assert_eq!(
        outcome.unfinished,
        Some(Unfinished::Services(vec!["stuck".into()]))
    );
}

#[tokio::test(start_paused = true)]
async fn cleanup_uses_remaining_budget_and_repeated_requests_do_not_extend_it() {
    let mut lifecycle = coordinator(5);
    let shutdown = lifecycle.shutdown();
    let participant = shutdown.clone();
    lifecycle
        .service("http", async move {
            participant.requested().await;
            tokio::time::sleep(Duration::from_secs(3)).await;
            participant.request();
            Ok::<_, io::Error>(())
        })
        .unwrap();
    shutdown.request();
    let start = Instant::now();
    let outcome = report(
        lifecycle
            .run(no_trigger(), async {
                tokio::time::sleep(Duration::from_secs(3)).await;
                Ok::<_, io::Error>(())
            })
            .await
            .unwrap_err(),
    );
    assert_eq!(Instant::now() - start, Duration::from_secs(5));
    assert_eq!(outcome.unfinished, Some(Unfinished::Cleanup));
}

#[tokio::test]
async fn trigger_and_cleanup_errors_are_both_preserved() {
    let mut lifecycle = coordinator(5);
    let shutdown = lifecycle.shutdown();
    lifecycle
        .service("http", async move {
            shutdown.requested().await;
            Ok::<_, io::Error>(())
        })
        .unwrap();
    let outcome = report(
        lifecycle
            .run(
                async { Err::<ShutdownReason, _>(io::Error::other("trigger")) },
                async { Err(io::Error::other("cleanup")) },
            )
            .await
            .unwrap_err(),
    );
    assert_eq!(outcome.reason, ShutdownReason::TriggerFailed);
    assert_eq!(outcome.failures.len(), 2);
    assert_eq!(outcome.failures[0].phase, Phase::Trigger);
    assert_eq!(outcome.failures[1].phase, Phase::Cleanup);
}

#[tokio::test]
async fn zero_budget_does_not_poll_services_or_cleanup_after_request() {
    let mut lifecycle = coordinator(0);
    lifecycle.shutdown().request();
    lifecycle
        .service("http", async {
            panic!("zero budget should not poll");
            #[allow(unreachable_code)]
            Ok::<_, io::Error>(())
        })
        .unwrap();
    let outcome = report(lifecycle.run(no_trigger(), cleanup()).await.unwrap_err());
    assert_eq!(
        outcome.unfinished,
        Some(Unfinished::Services(vec!["http".into()]))
    );
}

#[tokio::test]
async fn invalid_configuration_is_rejected() {
    assert!(matches!(
        coordinator(1).run(no_trigger(), cleanup()).await,
        Err(LifecycleError::Configuration(
            ConfigurationError::NoServices
        ))
    ));
    let mut lifecycle = coordinator(1);
    lifecycle.service("same", cleanup()).unwrap();
    assert_eq!(
        lifecycle.service("same", cleanup()),
        Err(ConfigurationError::DuplicateService("same".into()))
    );
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::MAX,
    });
    lifecycle.service("service", cleanup()).unwrap();
    assert!(matches!(
        lifecycle.run(no_trigger(), cleanup()).await,
        Err(LifecycleError::Configuration(
            ConfigurationError::InvalidGracePeriod
        ))
    ));
}
