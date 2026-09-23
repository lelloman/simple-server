#![cfg(feature = "auth")]
use http::{
    HeaderMap, HeaderValue, Request, Response, StatusCode, header::AUTHORIZATION, request::Parts,
};
use simple_server::auth::{
    Access, AsyncAccess, AuthLayer, CredentialError, HeaderCredential, Identity, RepeatedHeaders,
    SchemeCase,
};
use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use tower::{Layer, Service, ServiceExt, service_fn};

#[test]
fn credentials_preserve_bytes_and_reject_ambiguity_without_leaking_secrets() {
    let parser =
        HeaderCredential::new(AUTHORIZATION).with_scheme("Bearer", SchemeCase::AsciiInsensitive);
    let mut headers = HeaderMap::new();
    assert_eq!(
        parser.extract(&headers).unwrap_err(),
        CredentialError::Missing
    );
    headers.insert(AUTHORIZATION, HeaderValue::from_static("bEaReR secret "));
    let credential = parser.extract(&headers).unwrap();
    assert_eq!(credential.expose(), "secret ");
    assert!(!format!("{credential:?}").contains("secret"));
    headers.append(AUTHORIZATION, HeaderValue::from_static("Bearer second"));
    assert_eq!(
        parser.extract(&headers).unwrap_err(),
        CredentialError::Repeated
    );
    assert_eq!(
        parser
            .clone()
            .repeated(RepeatedHeaders::First)
            .extract(&headers)
            .unwrap()
            .expose(),
        "secret "
    );
    headers.clear();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer "));
    assert_eq!(
        parser.extract(&headers).unwrap_err(),
        CredentialError::Empty
    );
    assert_eq!(
        parser
            .clone()
            .allow_empty(true)
            .extract(&headers)
            .unwrap()
            .expose(),
        ""
    );
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer\tsecret"));
    assert_eq!(
        parser.extract(&headers).unwrap_err(),
        CredentialError::InvalidScheme
    );
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_bytes(b"Bearer \xff").unwrap(),
    );
    assert_eq!(
        parser.extract(&headers).unwrap_err(),
        CredentialError::InvalidText
    );
}

#[test]
fn sync_flow_requires_verification_and_all_checks_in_order() {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let access = Access::new(|value: &i32| {
        if *value < 0 {
            Err("unauthenticated")
        } else {
            Ok(*value)
        }
    })
    .with_check(move |p, _| {
        observed.fetch_add(1, Ordering::SeqCst);
        if *p == 0 { Err("forbidden") } else { Ok(()) }
    })
    .with_check(|_, _| Err("resource hidden"));
    assert_eq!(access.evaluate(&-1), Err("unauthenticated"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(access.evaluate(&0), Err("forbidden"));
    assert_eq!(access.evaluate(&1), Err("resource hidden"));
    assert_eq!(count.load(Ordering::SeqCst), 2);
    let anonymous = Access::<(), Option<u64>, &str>::new(|_| Ok(None));
    assert_eq!(anonymous.evaluate(&()), Ok(None)); // Anonymous is explicit app policy.
}

fn access() -> AsyncAccess<Parts, String, StatusCode> {
    AsyncAccess::new(|parts: &Parts| {
        Box::pin(async move {
            match parts.headers.get("x-user").and_then(|v| v.to_str().ok()) {
                Some("outage") => Err(StatusCode::SERVICE_UNAVAILABLE),
                Some(user) => Ok(user.to_owned()),
                None => Err(StatusCode::UNAUTHORIZED),
            }
        })
    })
    .with_check(|user, parts| {
        Box::pin(async move {
            if parts.uri.path() == "/admin" && user != "admin" {
                Err(StatusCode::FORBIDDEN)
            } else {
                Ok(())
            }
        })
    })
}
fn reject(status: StatusCode) -> Response<String> {
    Response::builder()
        .status(status)
        .header("cache-control", "no-store")
        .body("denied".into())
        .unwrap()
}

#[tokio::test]
async fn http_gate_preserves_body_context_identity_and_failure_responses() {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let inner = service_fn(move |request: Request<String>| {
        seen.fetch_add(1, Ordering::SeqCst);
        async move {
            let identity = request.extensions().get::<Identity<String>>().unwrap();
            assert_eq!(request.extensions().get::<u32>(), Some(&42));
            Ok::<_, Infallible>(Response::new(format!(
                "{}:{}",
                identity.principal(),
                request.body()
            )))
        }
    });
    let service = AuthLayer::new(access(), reject).layer(inner);
    for (user, path, expected) in [
        (None, "/admin", 401),
        (Some("member"), "/admin", 403),
        (Some("outage"), "/", 503),
        (Some("admin"), "/admin", 200),
        (Some("member"), "/", 200),
    ] {
        let mut request = Request::builder()
            .uri(path)
            .body("stream untouched".to_owned())
            .unwrap();
        request.extensions_mut().insert(42_u32);
        if let Some(user) = user {
            request
                .headers_mut()
                .insert("x-user", user.parse().unwrap());
        }
        let response = service.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status().as_u16(), expected);
        if expected == 200 {
            assert!(response.body().ends_with(":stream untouched"));
        } else {
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(response.body(), "denied");
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

struct ReadyService(bool);
impl Clone for ReadyService {
    fn clone(&self) -> Self {
        Self(false)
    }
}
impl Service<Request<()>> for ReadyService {
    type Response = Response<String>;
    type Error = Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0 = true;
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, _: Request<()>) -> Self::Future {
        assert!(self.0, "unready clone called");
        self.0 = false;
        std::future::ready(Ok(Response::new("ok".into())))
    }
}
#[tokio::test]
async fn gate_calls_the_ready_instance_and_can_be_reused_after_denial() {
    let mut service = AuthLayer::new(access(), reject).layer(ReadyService(false));
    for user in [None, Some("admin"), Some("member")] {
        let mut req = Request::new(());
        if let Some(user) = user {
            req.headers_mut().insert("x-user", user.parse().unwrap());
        }
        let result = service.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(
            result.status(),
            if user.is_some() {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        );
    }
}

#[tokio::test]
async fn cancellation_stops_checks_and_never_invokes_handler() {
    let later = Arc::new(AtomicUsize::new(0));
    let count = later.clone();
    let policy =
        AsyncAccess::new(|_: &Parts| Box::pin(async { Ok::<_, StatusCode>("user".to_owned()) }))
            .with_check(|_, _| Box::pin(std::future::pending()))
            .with_check(move |_, _| {
                count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(()) })
            });
    let inner = service_fn(|_: Request<()>| async {
        panic!("unauthorized dispatch");
        #[allow(unreachable_code)]
        Ok::<_, Infallible>(Response::new(String::new()))
    });
    let service = AuthLayer::new(policy, reject).layer(inner);
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(10),
            service.oneshot(Request::new(()))
        )
        .await
        .is_err()
    );
    assert_eq!(later.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn stacked_gate_cannot_reuse_a_previous_identity() {
    let outer = access();
    let inner_policy = AsyncAccess::new(|parts: &Parts| {
        Box::pin(async move {
            assert!(parts.extensions.get::<Identity<String>>().is_none());
            Err::<String, _>(StatusCode::UNAUTHORIZED)
        })
    });
    let endpoint = service_fn(|_: Request<()>| async {
        panic!("stale identity admitted");
        #[allow(unreachable_code)]
        Ok::<_, Infallible>(Response::new(String::new()))
    });
    let service =
        AuthLayer::new(outer, reject).layer(AuthLayer::new(inner_policy, reject).layer(endpoint));
    let request = Request::builder()
        .header("x-user", "admin")
        .body(())
        .unwrap();
    assert_eq!(
        service.oneshot(request).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn cloned_flows_revalidate_each_request_and_do_not_cache_permissions() {
    use std::sync::atomic::AtomicBool;
    let enabled = Arc::new(AtomicBool::new(true));
    let state = enabled.clone();
    let policy = AsyncAccess::new(|user: &String| {
        Box::pin(async move { Ok::<_, &'static str>(user.clone()) })
    })
    .with_check(move |_, _| {
        let state = state.clone();
        Box::pin(async move {
            if state.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err("revoked")
            }
        })
    });
    let a = "alice".to_owned();
    let b = "bob".to_owned();
    let clone = policy.clone();
    let (left, right) = tokio::join!(policy.evaluate(&a), clone.evaluate(&b));
    assert_eq!(left.unwrap(), "alice");
    assert_eq!(right.unwrap(), "bob");
    enabled.store(false, Ordering::SeqCst);
    assert_eq!(policy.evaluate(&a).await, Err("revoked"));
}
