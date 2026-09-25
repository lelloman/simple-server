#![cfg(feature = "auth")]
use http::{
    HeaderMap, HeaderValue, Request, Response, StatusCode,
    header::{AUTHORIZATION, COOKIE},
};
use simple_server::auth::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::{Layer, ServiceExt, service_fn};

fn cookie() -> CookieCredential {
    CookieCredential::new("session").unwrap()
}
fn header() -> HeaderCredential {
    HeaderCredential::new(AUTHORIZATION).with_scheme("Bearer", SchemeCase::AsciiInsensitive)
}
fn sources() -> CredentialSources {
    CredentialSources::header(header()).or_cookie(cookie())
}
fn headers(auth: Option<&str>, cookies: &[&str]) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Some(auth) = auth {
        h.insert(AUTHORIZATION, auth.parse().unwrap());
    }
    for c in cookies {
        h.append(COOKIE, c.parse().unwrap());
    }
    h
}

#[test]
fn cookie_parsing_preserves_opaque_values_and_checks_all_headers() {
    let h = headers(None, &["other=one; Session=wrong", "session=abc%2F=="]);
    assert_eq!(cookie().extract(&h).unwrap().expose(), "abc%2F==");
    let h = headers(None, &["unrelated; session=\"quoted\""]);
    assert_eq!(cookie().extract(&h).unwrap().expose(), "quoted");
    assert_eq!(
        cookie().extract(&headers(None, &[])).unwrap_err(),
        CookieCredentialError::Missing
    );
    let h = headers(None, &["session=a", "session=b"]);
    assert_eq!(
        cookie().extract(&h).unwrap_err(),
        CookieCredentialError::Repeated
    );
    assert_eq!(
        cookie()
            .repeated(RepeatedCookies::First)
            .extract(&h)
            .unwrap()
            .expose(),
        "a"
    );
    assert_eq!(
        cookie()
            .repeated(RepeatedCookies::Last)
            .extract(&h)
            .unwrap()
            .expose(),
        "b"
    );
    assert!(
        cookie()
            .allow_empty(true)
            .extract(&headers(None, &["session="]))
            .unwrap()
            .expose()
            .is_empty()
    );
}

#[test]
fn bad_cookie_names_and_values_fail_without_exposing_secrets() {
    for name in ["", "a b", "a=b", "a;b", "a\r\n", "é"] {
        assert!(CookieCredential::new(name).is_err());
    }
    for (value, expected) in [
        ("session", CookieCredentialError::Malformed),
        ("session =secret", CookieCredentialError::Malformed),
        ("session=", CookieCredentialError::Empty),
        ("session=has space", CookieCredentialError::Malformed),
        ("session=\"unclosed", CookieCredentialError::Malformed),
        ("session=secret\\value", CookieCredentialError::Malformed),
    ] {
        let e = cookie().extract(&headers(None, &[value])).unwrap_err();
        assert_eq!(e, expected);
        assert!(!format!("{e:?} {e}").contains("secret"));
    }
    let mut h = HeaderMap::new();
    h.insert(COOKIE, HeaderValue::from_bytes(b"session=\xff").unwrap());
    assert_eq!(
        cookie().extract(&h).unwrap_err(),
        CookieCredentialError::InvalidText
    );
    let chosen = sources()
        .extract(&headers(Some("Bearer TOP_SECRET"), &[]))
        .unwrap();
    assert!(!format!("{chosen:?}").contains("TOP_SECRET"));
    assert!(!format!("{:?}", CredentialAuthError::Access("TOP_SECRET")).contains("TOP_SECRET"));
}

#[test]
fn source_order_missing_and_malformed_fallback_are_explicit() {
    let h = headers(Some("Bearer header"), &["session=cookie"]);
    let selected = sources().extract(&h).unwrap();
    assert_eq!(selected.expose(), "header");
    assert_eq!(selected.source(), &CredentialSource::Header(AUTHORIZATION));
    assert_eq!(
        CredentialSources::cookie(cookie())
            .or_header(header())
            .extract(&h)
            .unwrap()
            .expose(),
        "cookie"
    );
    assert_eq!(
        sources()
            .extract(&headers(None, &["session=cookie"]))
            .unwrap()
            .source(),
        &CredentialSource::Cookie("session".into())
    );
    let h = headers(Some("Basic invalid"), &["session=cookie"]);
    assert!(matches!(
        sources().extract(&h),
        Err(CredentialSelectionError::Header(_))
    ));
    assert_eq!(
        sources()
            .on_malformed(MalformedCredentials::TryNext)
            .extract(&h)
            .unwrap()
            .expose(),
        "cookie"
    );
    assert!(matches!(
        sources()
            .on_malformed(MalformedCredentials::TryNext)
            .extract(&headers(Some("Basic invalid"), &[])),
        Err(CredentialSelectionError::Header(_))
    ));
    // A selected higher-priority credential ignores malformed lower sources.
    assert_eq!(
        sources()
            .extract(&headers(Some("Bearer header"), &["session="]))
            .unwrap()
            .expose(),
        "header"
    );
    let mut duplicate = headers(Some("Bearer one"), &["session=cookie"]);
    duplicate.append(AUTHORIZATION, "Bearer two".parse().unwrap());
    assert!(matches!(
        sources().extract(&duplicate),
        Err(CredentialSelectionError::Header(CredentialError::Repeated))
    ));
}

fn verifier(calls: Arc<AtomicUsize>) -> AsyncAccess<SelectedCredential, String, &'static str> {
    AsyncAccess::new(move |c: &SelectedCredential| {
        calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            match c.expose() {
                "valid" => Ok("user".into()),
                "outage" => Err("unavailable"),
                _ => Err("invalid"),
            }
        })
    })
}
fn reject(error: CredentialAuthError<&'static str>) -> Response<String> {
    let status = match error {
        CredentialAuthError::Access("unavailable") => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::UNAUTHORIZED,
    };
    Response::builder()
        .status(status)
        .body("denied".into())
        .unwrap()
}

#[tokio::test]
async fn admission_matrix_preserves_sources_bodies_and_errors() {
    for optional in [false, true] {
        for (auth, cookies, status, source, calls_expected) in [
            (
                None,
                vec![],
                if optional {
                    StatusCode::OK
                } else {
                    StatusCode::UNAUTHORIZED
                },
                None,
                0,
            ),
            (
                None,
                vec!["session=valid"],
                StatusCode::OK,
                Some(CredentialSource::Cookie("session".into())),
                1,
            ),
            (
                Some("Bearer valid"),
                vec!["session=invalid"],
                StatusCode::OK,
                Some(CredentialSource::Header(AUTHORIZATION)),
                1,
            ),
            (
                Some("Basic bad"),
                vec!["session=valid"],
                StatusCode::UNAUTHORIZED,
                None,
                0,
            ),
            (
                Some("Bearer invalid"),
                vec!["session=valid"],
                StatusCode::UNAUTHORIZED,
                None,
                1,
            ),
            (None, vec!["session="], StatusCode::UNAUTHORIZED, None, 0),
            (
                None,
                vec!["session=valid", "session=valid"],
                StatusCode::UNAUTHORIZED,
                None,
                0,
            ),
            (
                Some("Bearer outage"),
                vec!["session=valid"],
                StatusCode::SERVICE_UNAVAILABLE,
                None,
                1,
            ),
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let endpoint = service_fn(move |r: Request<String>| {
                assert_eq!(r.body(), "unconsumed body");
                assert_eq!(r.headers()["x-keep"], "yes");
                assert_eq!(r.extensions().get::<usize>(), Some(&42));
                let identity = r.extensions().get::<Identity<String>>();
                assert_eq!(identity.and_then(Identity::source), source.as_ref());
                assert_eq!(
                    identity.map(|i| i.principal().as_str()),
                    source.as_ref().map(|_| "user")
                );
                async { Ok::<_, std::convert::Infallible>(Response::new("ok".to_owned())) }
            });
            let layer = AuthLayer::credentials(
                sources(),
                if optional {
                    Authentication::Optional
                } else {
                    Authentication::Required
                },
                verifier(calls.clone()),
                reject,
            );
            let mut request = Request::new("unconsumed body".to_owned());
            *request.headers_mut() = headers(auth, &cookies);
            request
                .headers_mut()
                .insert("x-keep", "yes".parse().unwrap());
            request.extensions_mut().insert(42usize);
            assert_eq!(
                layer
                    .layer(endpoint)
                    .oneshot(request)
                    .await
                    .unwrap()
                    .status(),
                status
            );
            assert_eq!(calls.load(Ordering::SeqCst), calls_expected);
        }
    }
}

#[tokio::test]
async fn fallback_never_retries_failed_verification_or_authorization() {
    let calls = Arc::new(AtomicUsize::new(0));
    let access = verifier(calls.clone()).with_check(|_, _| Box::pin(async { Err("forbidden") }));
    let service = AuthLayer::credentials(
        sources().on_malformed(MalformedCredentials::TryNext),
        Authentication::Optional,
        access,
        reject,
    )
    .layer(service_fn(|_: Request<()>| async {
        panic!("denied request reached handler");
        #[allow(unreachable_code)]
        Ok::<_, std::convert::Infallible>(Response::new(String::new()))
    }));
    let mut request = Request::new(());
    *request.headers_mut() = headers(Some("Bearer valid"), &["session=valid"]);
    assert_eq!(
        service.oneshot(request).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn anonymous_inner_gate_clears_outer_identity_and_source() {
    let outer = AuthLayer::credentials(
        CredentialSources::cookie(cookie()),
        Authentication::Required,
        verifier(Arc::default()),
        reject,
    );
    let inner = AuthLayer::credentials(
        CredentialSources::header(header()),
        Authentication::Optional,
        verifier(Arc::default()),
        reject,
    );
    let endpoint = service_fn(|r: Request<()>| async move {
        assert!(r.extensions().get::<Identity<String>>().is_none());
        Ok::<_, std::convert::Infallible>(Response::new("anonymous".to_owned()))
    });
    let mut request = Request::new(());
    *request.headers_mut() = headers(None, &["session=valid"]);
    assert_eq!(
        outer
            .layer(inner.layer(endpoint))
            .oneshot(request)
            .await
            .unwrap()
            .body(),
        "anonymous"
    );
}

#[cfg(feature = "auth-cookies")]
#[test]
fn decoded_cookie_compatibility_matches_cookie_jar_and_keeps_strict_default() {
    for lines in [
        vec!["session=%61%2F%3D; other=x"],
        vec!["session=first", "session=last"],
        vec![" malformed ; session = spaced ; other=x"],
        vec!["session=valid; session="],
        vec!["ses%73ion=encoded-name"],
        vec!["session=\"quoted\""],
        vec!["session=%FF"],
        vec!["session=valid; broken"],
        vec!["session=plus+literal"],
    ] {
        let h = headers(None, &lines);
        let mut oracle = cookie::CookieJar::new();
        for pair in h
            .get_all(COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
        {
            if let Ok(cookie) = cookie::Cookie::parse_encoded(pair.to_owned()) {
                oracle.add_original(cookie);
            }
        }
        let parsed = CookieCredential::decoded_compatibility("session")
            .unwrap()
            .extract(&h)
            .ok();
        assert_eq!(
            parsed.as_ref().map(|v| v.expose()),
            oracle.get("session").map(|v| v.value()),
            "{lines:?}"
        );
    }
    let mut h = headers(None, &["session=%61"]);
    h.append(COOKIE, HeaderValue::from_bytes(b"session=\xff").unwrap());
    assert_eq!(
        CookieCredential::decoded_compatibility("session")
            .unwrap()
            .extract(&h)
            .unwrap()
            .expose(),
        "a"
    );
    assert_eq!(
        cookie().extract(&h).unwrap_err(),
        CookieCredentialError::InvalidText
    );
    assert_eq!(
        cookie()
            .extract(&headers(None, &["session=%61"]))
            .unwrap()
            .expose(),
        "%61"
    );
    assert_eq!(
        CookieCredential::decoded_compatibility("session")
            .unwrap()
            .repeated(RepeatedCookies::Reject)
            .extract(&headers(None, &["session=one; session=two"]))
            .unwrap_err(),
        CookieCredentialError::Repeated
    );
    let secret = CookieCredential::decoded_compatibility("session")
        .unwrap()
        .extract(&headers(None, &["session=TOP_SECRET"]))
        .unwrap()
        .expose()
        .to_owned();
    assert_eq!(secret, "TOP_SECRET");
}

#[tokio::test]
async fn lazy_authentication_has_same_policy_and_clears_stale_identity() {
    let calls = Arc::new(AtomicUsize::new(0));
    let layer = AuthLayer::credentials(
        sources(),
        Authentication::Optional,
        verifier(calls.clone()),
        reject,
    );
    let (mut parts, _) = Request::new(()).into_parts();
    parts.headers = headers(None, &["session=valid"]);
    parts.extensions.insert(42usize);
    let identity = layer.authenticate(&mut parts).await.unwrap().unwrap();
    assert_eq!(
        identity.source(),
        Some(&CredentialSource::Cookie("session".into()))
    );
    parts.extensions.insert(identity);
    parts.headers = headers(None, &[]);
    assert!(layer.authenticate(&mut parts).await.unwrap().is_none());
    assert!(parts.extensions.get::<Identity<String>>().is_none());
    assert_eq!(parts.extensions.get::<usize>(), Some(&42));
    parts.headers = headers(Some("Bearer invalid"), &["session=valid"]);
    assert!(matches!(
        layer.authenticate(&mut parts).await,
        Err(CredentialAuthError::Access("invalid"))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
