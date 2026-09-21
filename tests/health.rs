#![cfg(feature = "health")]

use http::{Request, Response, StatusCode};
use simple_server::health::{Check, Probe};
use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tower::ServiceExt;

#[tokio::test]
async fn ordered_checks_stop_at_first_error_and_recover_without_caching() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let failing = Arc::new(AtomicBool::new(true));
    let make_check = |name: &'static str| {
        let calls = calls.clone();
        let failing = failing.clone();
        Check::new(name, move || {
            let calls = calls.clone();
            let failing = failing.clone();
            async move {
                calls.lock().unwrap().push(name);
                if name == "database" && failing.load(Ordering::SeqCst) {
                    Err("original error")
                } else {
                    Ok(())
                }
            }
        })
    };
    let probe = Probe::readiness(make_check("database"))
        .with_check(make_check("storage"))
        .with_check(make_check("tools"));
    let failure = probe.run().await.unwrap_err();
    assert_eq!(&*failure.name, "database");
    assert_eq!(failure.error, "original error");
    assert_eq!(*calls.lock().unwrap(), ["database"]);
    calls.lock().unwrap().clear();
    failing.store(false, Ordering::SeqCst);
    probe.clone().run().await.unwrap();
    assert_eq!(*calls.lock().unwrap(), ["database", "storage", "tools"]);
    Probe::liveness().run().await.unwrap();
}

#[tokio::test]
async fn endpoint_preserves_application_response_and_non_clone_errors() {
    struct Error;
    let probe = Probe::readiness(Check::new("dependency", || async { Err::<(), _>(Error) }));
    let endpoint = probe.endpoint(|result| {
        assert_eq!(&*result.unwrap_err().name, "dependency");
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("x-app-policy", "kept")
            .body("private application format")
            .unwrap()
    });
    let response = endpoint.clone().oneshot(Request::new(())).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.headers()["x-app-policy"], "kept");
    assert_eq!(response.body(), &"private application format");
}

#[tokio::test]
async fn cancelling_a_probe_drops_active_check_without_starting_next() {
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let probe = Probe::readiness(Check::<Infallible>::new("pending", {
        let dropped = dropped.clone();
        move || {
            let guard = Guard(dropped.clone());
            async move {
                let _guard = guard;
                std::future::pending().await
            }
        }
    }))
    .with_check(Check::new("never", || async { panic!("must not run") }));
    let mut future = Box::pin(probe.run());
    assert!(futures_util::poll!(&mut future).is_pending());
    drop(future);
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn concurrent_requests_do_not_serialize_or_cache_checks() {
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let probe = Probe::readiness(Check::<Infallible>::new("barrier", move || {
        let barrier = barrier.clone();
        async move {
            barrier.wait().await;
            Ok(())
        }
    }));
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        let (a, b) = tokio::join!(probe.run(), probe.run());
        a.unwrap();
        b.unwrap();
    })
    .await
    .unwrap();
}

#[cfg(feature = "http")]
#[tokio::test]
async fn router_owns_get_head_and_method_rejection() {
    use simple_server::axum::{
        Router,
        body::{Body, to_bytes},
        routing::get_service,
    };
    let endpoint = Probe::liveness().endpoint(|result| {
        result.unwrap();
        Response::builder()
            .header("content-type", "application/json")
            .body(Body::from("{\"status\":\"ok\"}"))
            .unwrap()
    });
    let app = Router::new().route("/health", get_service(endpoint));
    for (method, status, body) in [
        ("GET", 200, "{\"status\":\"ok\"}"),
        ("HEAD", 200, ""),
        ("POST", 405, ""),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        if method != "POST" {
            assert_eq!(response.headers()["content-type"], "application/json");
        }
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            body
        );
    }
}

#[tokio::test]
async fn value_checks_preserve_full_aggregate_reports_on_both_outcomes() {
    #[derive(Debug, PartialEq)]
    struct Report {
        engines: Vec<bool>,
    }
    let healthy = Arc::new(AtomicBool::new(true));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let check = Check::new("engines", {
        let healthy = healthy.clone();
        let calls = calls.clone();
        move || {
            let healthy = healthy.clone();
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                let report = Report {
                    engines: vec![false, healthy.load(Ordering::SeqCst), false],
                };
                if report.engines.iter().any(|ready| *ready) {
                    Ok(report)
                } else {
                    Err(report)
                }
            }
        }
    });
    assert_eq!(check.run().await.unwrap().engines, [false, true, false]);
    healthy.store(false, Ordering::SeqCst);
    let failure = check.clone().run().await.unwrap_err();
    assert_eq!(&*failure.name, "engines");
    assert_eq!(failure.error.engines, [false, false, false]);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
