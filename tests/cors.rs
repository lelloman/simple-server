#![cfg(feature = "cors")]

use http::{HeaderValue, Method, Request, Response, header::*};
use simple_server::cors::CorsConfig;
use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tower::{Layer, ServiceExt, service_fn};
use tower_http::cors::{AllowOrigin, Any, CorsLayer as Legacy};

fn configured() -> CorsConfig {
    CorsConfig::default()
        .allow_origins([HeaderValue::from_static("https://app.example")])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            http::HeaderName::from_static("x-csrf-token"),
        ])
        .expose_headers([http::HeaderName::from_static("x-correlation-id")])
        .allow_credentials(true)
        .max_age(Duration::from_secs(300))
}
fn legacy_configured() -> Legacy {
    Legacy::new()
        .allow_origin(AllowOrigin::list([HeaderValue::from_static(
            "https://app.example",
        )]))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            http::HeaderName::from_static("x-csrf-token"),
        ])
        .expose_headers([http::HeaderName::from_static("x-correlation-id")])
        .allow_credentials(true)
        .max_age(Duration::from_secs(300))
}
#[tokio::test]
async fn legacy_equivalence_for_policies_origins_methods_status_and_existing_headers() {
    let any = CorsConfig::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header();
    for (policy, legacy) in [
        (CorsConfig::default(), Legacy::new()),
        (
            CorsConfig::default().allow_credentials(true),
            Legacy::new().allow_credentials(true),
        ),
        (configured(), legacy_configured()),
        (
            any.clone(),
            Legacy::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        ),
        (any.expose_any_header(), Legacy::permissive()),
    ] {
        for origin in [
            None,
            Some("https://app.example"),
            Some("https://evil.example"),
            Some("null"),
        ] {
            for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
                for status in [200, 401, 404, 413, 500] {
                    let request = || {
                        let mut r = Request::builder()
                            .method(method.clone())
                            .header(ACCESS_CONTROL_REQUEST_METHOD, "POST")
                            .header(ACCESS_CONTROL_REQUEST_HEADERS, "content-type,x-csrf-token");
                        if let Some(o) = origin {
                            r = r.header(ORIGIN, o);
                        }
                        r.body(()).unwrap()
                    };
                    let calls = Arc::new(AtomicUsize::new(0));
                    let service = service_fn({
                        let calls = calls.clone();
                        move |_| {
                            calls.fetch_add(1, Ordering::SeqCst);
                            async move {
                                let mut response = Response::builder()
                                    .status(status)
                                    .header(VARY, "accept-encoding")
                                    .header(VARY, "x-app")
                                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "https://inner.example")
                                    .header("x-correlation-id", "request-123")
                                    .body("unchanged body".to_owned())
                                    .unwrap();
                                response.extensions_mut().insert(42usize);
                                Ok::<_, Infallible>(response)
                            }
                        }
                    });
                    let expected = legacy
                        .layer(service.clone())
                        .oneshot(request())
                        .await
                        .unwrap();
                    let actual = policy
                        .clone()
                        .build()
                        .unwrap()
                        .layer(service)
                        .oneshot(request())
                        .await
                        .unwrap();
                    assert_eq!(actual.status(), expected.status());
                    assert_eq!(actual.headers(), expected.headers());
                    assert_eq!(
                        actual.extensions().get::<usize>(),
                        expected.extensions().get::<usize>()
                    );
                    assert_eq!(actual.body(), expected.body());
                    assert_eq!(
                        calls.load(Ordering::SeqCst),
                        if method == Method::OPTIONS { 0 } else { 2 }
                    );
                }
            }
        }
    }
}

#[test]
fn invalid_policies_fail_at_build_and_setters_replace() {
    for policy in [
        CorsConfig::default().allow_any_origin(),
        CorsConfig::default().allow_any_method(),
        CorsConfig::default().allow_any_header(),
        CorsConfig::default().expose_any_header(),
    ] {
        assert!(policy.clone().allow_credentials(true).build().is_err());
        assert!(
            policy
                .allow_credentials(true)
                .allow_credentials(false)
                .build()
                .is_ok()
        );
    }
    assert!(
        CorsConfig::default()
            .allow_credentials(true)
            .allow_any_origin()
            .build()
            .is_err()
    );
    assert!(
        CorsConfig::default()
            .allow_origins([HeaderValue::from_static("*")])
            .build()
            .is_err()
    );
    assert!(
        CorsConfig::default()
            .allow_methods([Method::from_bytes(b"*").unwrap()])
            .build()
            .is_err()
    );
    assert!(
        CorsConfig::default()
            .allow_headers([http::HeaderName::from_static("*")])
            .build()
            .is_err()
    );
    assert!(
        CorsConfig::default()
            .expose_headers([http::HeaderName::from_static("*")])
            .build()
            .is_err()
    );
    assert!(
        CorsConfig::default()
            .allow_any_origin()
            .allow_origins([])
            .allow_credentials(true)
            .build()
            .is_ok()
    );
}

#[tokio::test]
async fn no_default_permissions_and_errors_pass_through() {
    let layer = CorsConfig::default().build().unwrap();
    let response = layer
        .layer(service_fn(|_: Request<()>| async {
            Ok::<_, Infallible>(Response::new(()))
        }))
        .oneshot(
            Request::builder()
                .header(ORIGIN, "https://app.example")
                .body(())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    assert!(
        !response
            .headers()
            .contains_key(ACCESS_CONTROL_ALLOW_CREDENTIALS)
    );
    let failed = layer
        .layer(service_fn(|_: Request<()>| async {
            Err::<Response<()>, _>("original error")
        }))
        .oneshot(Request::new(()))
        .await;
    assert_eq!(failed.unwrap_err(), "original error");
}

#[tokio::test]
async fn streams_are_never_polled_by_cors_and_extensions_survive() {
    // The layer accepts any defaultable body: it cannot poll, clone or inspect
    // this opaque application-owned stream state.
    #[derive(Default)]
    struct UntouchedBody(Option<Arc<usize>>);
    let marker = Arc::new(123);
    let expected = marker.clone();
    let service = service_fn(move |_: Request<()>| {
        let marker = marker.clone();
        async move {
            let mut response = Response::new(UntouchedBody(Some(marker)));
            response.extensions_mut().insert(456usize);
            Ok::<_, Infallible>(response)
        }
    });
    let response = configured()
        .build()
        .unwrap()
        .layer(service)
        .oneshot(Request::new(()))
        .await
        .unwrap();
    assert!(Arc::ptr_eq(response.body().0.as_ref().unwrap(), &expected));
    assert_eq!(response.extensions().get::<usize>(), Some(&456));
}

#[tokio::test]
async fn readiness_and_readiness_errors_are_forwarded() {
    use std::{
        future::{Ready, ready},
        task::{Context, Poll},
    };
    use tower::Service;
    struct Unready(bool);
    impl Service<Request<()>> for Unready {
        type Response = Response<()>;
        type Error = &'static str;
        type Future = Ready<Result<Response<()>, Self::Error>>;
        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if !self.0 {
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(Err("readiness error"))
            }
        }
        fn call(&mut self, _: Request<()>) -> Self::Future {
            ready(Ok(Response::new(())))
        }
    }
    let mut service = configured().build().unwrap().layer(Unready(false));
    assert!(matches!(service.ready().await, Err("readiness error")));
}
