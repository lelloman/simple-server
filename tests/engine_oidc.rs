#![cfg(all(feature = "oidc", feature = "engine-web"))]
use serde_json::json;
use simple_server::{engine_lifecycle::Shutdown, engine_web as web, oidc};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn config(issuer: String, development: bool) -> oidc::Config {
    oidc::Config {
        issuer,
        client_id: "test-client".into(),
        client_secret: "test-secret".into(),
        redirect_uri: "https://service.example/callback".into(),
        allow_loopback_http: development,
    }
}
async fn provider(
    mode: &'static str,
) -> (
    String,
    Shutdown,
    simple_server::runtime::JoinHandle<()>,
    Arc<AtomicUsize>,
) {
    let listener = web::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr());
    let hits = Arc::new(AtomicUsize::new(0));
    let mut router = web::Router::new().unwrap();
    let issuer = origin.clone();
    let count = hits.clone();
    router = router.route("/.well-known/openid-configuration", web::MethodRouter::new().unwrap().on(web::Method::GET,move |_| {
        let issuer = issuer.clone(); let count = count.clone();
        async move {
            count.fetch_add(1,Ordering::SeqCst);
            if mode == "redirect" {
                return web::http::Response::builder().status(302).header("location",format!("{issuer}/redirect-target")).body(web::Body::empty()).unwrap();
            }
            let metadata = json!({"issuer":if mode=="issuer" {"https://wrong.example"} else {&issuer},
                "authorization_endpoint":if mode=="authorization" {"http://non-loopback.invalid/auth".into()} else {format!("{issuer}/authorize")},
                "token_endpoint":if mode=="token" {"http://non-loopback.invalid/token".into()} else {format!("{issuer}/token")},
                "jwks_uri":if mode=="jwks" {"http://non-loopback.invalid/jwks".into()} else {format!("{issuer}/jwks")},
                "response_types_supported":["code"],"subject_types_supported":["public"],"id_token_signing_alg_values_supported":["RS256"]});
            web::http::Response::builder().header("content-type","application/json").body(web::Body::from(metadata.to_string())).unwrap()
        }
    }).unwrap()).unwrap();
    for path in ["/jwks", "/redirect-target"] {
        let count = hits.clone();
        router = router
            .route(
                path,
                web::MethodRouter::new()
                    .unwrap()
                    .on(web::Method::GET, move |_| {
                        let count = count.clone();
                        async move {
                            count.fetch_add(1, Ordering::SeqCst);
                            web::http::Response::builder()
                                .header("content-type", "application/json")
                                .body(web::Body::from("{\"keys\":[]}"))
                                .unwrap()
                        }
                    })
                    .unwrap(),
            )
            .unwrap();
    }
    let stop = Shutdown::new();
    let shutdown = stop.clone();
    let task = simple_server::runtime::spawn(async move {
        web::serve(listener, router, shutdown).await.unwrap();
    });
    (origin, stop, task, hits)
}

#[simple_server::test]
async fn authorization_material_is_fresh_and_clones_retain_the_provider() {
    let (origin, stop, task, _) = provider("ok").await;
    let client = oidc::Client::discover(config(origin.clone(), true))
        .await
        .unwrap();
    let cloned = client.clone();
    drop(client);
    let first = cloned.begin().await.unwrap();
    let second = cloned.begin().await.unwrap();
    assert!(first.url.starts_with(&format!("{origin}/authorize?")));
    for required in [
        "response_type=code",
        "client_id=test-client",
        "scope=openid",
        "code_challenge_method=S256",
        "code_challenge=",
        "state=",
        "nonce=",
    ] {
        assert!(first.url.contains(required), "{required}");
    }
    assert_ne!(first.state, second.state);
    assert_ne!(first.nonce, second.nonce);
    assert_ne!(first.pkce_verifier, second.pkce_verifier);
    assert!(first.pkce_verifier.len() >= 43);
    assert!(!format!("{first:?}").contains(&first.pkce_verifier));
    assert!(!format!("{:?}", config(origin, true)).contains("test-secret"));
    assert_eq!(
        cloned
            .finish("", &first.nonce, &first.pkce_verifier)
            .await
            .unwrap_err(),
        oidc::Error::Unauthorized
    );
    drop(cloned);
    stop.request();
    task.await.unwrap();
}

#[simple_server::test]
async fn production_rejects_loopback_http_before_discovery() {
    let (origin, stop, task, hits) = provider("ok").await;
    assert_eq!(
        oidc::Client::discover(config(origin, false))
            .await
            .unwrap_err(),
        oidc::Error::UnsafeEndpoint
    );
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    stop.request();
    task.await.unwrap();
}

#[simple_server::test]
async fn discovery_rejects_redirects_issuer_mismatch_and_unsafe_endpoints() {
    for mode in ["redirect", "issuer", "authorization", "token", "jwks"] {
        let (origin, stop, task, hits) = provider(mode).await;
        let error = oidc::Client::discover(config(origin, true))
            .await
            .unwrap_err();
        assert!(
            matches!(error, oidc::Error::Discovery | oidc::Error::UnsafeEndpoint),
            "{mode}: {error}"
        );
        if mode == "redirect" || mode == "jwks" {
            assert_eq!(hits.load(Ordering::SeqCst), 1);
        }
        assert!(!error.to_string().contains("test-secret"));
        stop.request();
        task.await.unwrap();
    }
}

#[simple_server::test]
async fn invalid_configuration_is_rejected_without_network_io() {
    let (origin, stop, task, hits) = provider("ok").await;
    let mut missing = config(origin.clone(), true);
    missing.client_secret.clear();
    assert_eq!(
        oidc::Client::discover(missing).await.unwrap_err(),
        oidc::Error::Configuration
    );
    let mut redirect = config(origin, true);
    redirect.redirect_uri = "http://service.example/callback".into();
    assert_eq!(
        oidc::Client::discover(redirect).await.unwrap_err(),
        oidc::Error::UnsafeEndpoint
    );
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    stop.request();
    task.await.unwrap();
}
