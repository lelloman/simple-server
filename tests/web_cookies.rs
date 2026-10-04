#![cfg(feature = "cookies")]

use simple_server::web::{
    Body, FromRequestParts, HeaderMap, HeaderValue, Request, Response, Router,
    cookies::{Cookie, CookieManagerLayer, Cookies, SameSite, time},
    header,
    middleware::{Next, from_fn},
    routing::post,
};
use tower::{Layer, Service, ServiceExt};

trait Jar {
    fn get(&self, name: &str) -> Option<Cookie<'static>>;
    fn add(&self, cookie: Cookie<'static>);
    fn remove(&self, cookie: Cookie<'static>);
    fn list(&self) -> Vec<Cookie<'static>>;
}
impl Jar for Cookies {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.get(name)
    }
    fn add(&self, cookie: Cookie<'static>) {
        self.add(cookie);
    }
    fn remove(&self, cookie: Cookie<'static>) {
        self.remove(cookie);
    }
    fn list(&self) -> Vec<Cookie<'static>> {
        self.list()
    }
}
#[cfg(feature = "tower-cookies")]
impl Jar for tower_cookies::Cookies {
    fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.get(name).map(Cookie::into_owned)
    }
    fn add(&self, cookie: Cookie<'static>) {
        self.add(cookie);
    }
    fn remove(&self, cookie: Cookie<'static>) {
        self.remove(cookie);
    }
    fn list(&self) -> Vec<Cookie<'static>> {
        self.list().into_iter().map(Cookie::into_owned).collect()
    }
}

async fn exercise(jar: impl Jar, request: Request) -> Response {
    let mode = request.headers()["action"].to_str().unwrap();
    let original = jar.get("session").map(|c| c.value().to_owned());
    match mode {
        "read" => (),
        "add" => {
            jar.add(
                Cookie::build(("__Host-session", "new"))
                    .path("/")
                    .secure(true)
                    .http_only(true)
                    .same_site(SameSite::Lax)
                    .max_age(time::Duration::days(7))
                    .build(),
            );
            jar.add(
                Cookie::build(("csrf", "token"))
                    .path("/")
                    .secure(true)
                    .http_only(true)
                    .same_site(SameSite::Strict)
                    .build(),
            );
        }
        "replace" => {
            jar.add(Cookie::new("session", "first"));
            jar.add(Cookie::new("session", "last"));
        }
        "remove" => jar.remove(
            Cookie::build(("session", ""))
                .path("/")
                .domain("example.test")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Lax)
                .build(),
        ),
        "cancel" => {
            jar.add(Cookie::new("temporary", "one"));
            jar.remove(Cookie::new("temporary", ""));
        }
        "delete" => jar.add(
            Cookie::build(("__Host-session", ""))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Lax)
                .max_age(time::Duration::ZERO)
                .build(),
        ),
        "attributes" => jar.add(
            Cookie::build(("custom", "raw / value"))
                .path("/app")
                .domain("example.test")
                .secure(false)
                .http_only(false)
                .same_site(SameSite::None)
                .expires(time::OffsetDateTime::from_unix_timestamp(1_900_000_000).unwrap())
                .build(),
        ),
        "invalid" => {
            jar.add(Cookie::new("bad", "value\r\nInjected: yes"));
            jar.add(Cookie::new("good", "kept"));
        }
        _ => panic!("unknown action"),
    }
    let mut values: Vec<_> = jar
        .list()
        .into_iter()
        .map(|c| (c.name().to_owned(), c.value().to_owned()))
        .collect();
    values.sort();
    let request_body = request.into_body().collect(4096).await.unwrap();
    let mut response = Response::new(Body::from(
        serde_json::to_vec(&(original, values, request_body.to_vec())).unwrap(),
    ));
    *response.status_mut() = simple_server::web::StatusCode::ACCEPTED;
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_static("existing=keep; Path=/"),
    );
    response
        .headers_mut()
        .insert("x-marker", HeaderValue::from_static("preserved"));
    response.extensions_mut().insert(42_u32);
    response
}
async fn owned(cookies: Cookies, request: Request) -> Response {
    exercise(cookies, request).await
}
#[cfg(feature = "tower-cookies")]
async fn legacy(cookies: tower_cookies::Cookies, request: Request) -> Response {
    exercise(cookies, request).await
}

fn request(action: &str, incoming: &[HeaderValue]) -> Request {
    let mut r = Request::builder()
        .method("POST")
        .uri("/")
        .header("action", action)
        .body(Body::from("untouched body"))
        .unwrap();
    for v in incoming {
        r.headers_mut().append(header::COOKIE, v.clone());
    }
    r
}

fn normalized_deltas(response: &Response) -> Vec<String> {
    let mut values: Vec<_> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| {
            let mut cookie = Cookie::parse(v.to_str().unwrap().to_owned()).unwrap();
            if cookie.max_age() == Some(time::Duration::ZERO) && cookie.expires().is_some() {
                // Removal dates are wall-clock dependent. Assert expiry then normalize
                // only that date, retaining every other attribute for comparison.
                assert!(cookie.expires_datetime().unwrap() < time::OffsetDateTime::now_utc());
                cookie.set_expires(time::OffsetDateTime::UNIX_EPOCH);
            }
            cookie.to_string()
        })
        .collect();
    values.sort();
    values
}

#[cfg(feature = "tower-cookies")]
#[tokio::test]
async fn matches_legacy_parsing_mutations_attributes_and_response_metadata() {
    let old = Router::new()
        .route("/", post(legacy))
        .layer(tower_cookies::CookieManagerLayer::new());
    let new = Router::new()
        .route("/", post(owned))
        .layer(CookieManagerLayer::new());
    let cases = [
        vec![],
        vec![HeaderValue::from_static("session=old; obsolete=value")],
        vec![
            HeaderValue::from_static("session=first; session=second"),
            HeaderValue::from_static("session=last; other=ok"),
        ],
        vec![HeaderValue::from_static(
            " encoded=hello%20world; utf8=%E2%98%83; session=with%2Fslash; empty=",
        )],
        vec![HeaderValue::from_static(
            "bad=%FF; =missing; ; bare; quoted=\"hello\"; session=valid",
        )],
        vec![
            HeaderValue::from_bytes(b"session=\xff").unwrap(),
            HeaderValue::from_static("session=valid"),
        ],
    ];
    for incoming in cases {
        for action in [
            "read",
            "add",
            "replace",
            "remove",
            "cancel",
            "delete",
            "attributes",
            "invalid",
        ] {
            let a = old
                .clone()
                .oneshot(request(action, &incoming))
                .await
                .unwrap();
            let b = new
                .clone()
                .oneshot(request(action, &incoming))
                .await
                .unwrap();
            assert_eq!(a.status(), b.status(), "{action}");
            assert_eq!(normalized_deltas(&a), normalized_deltas(&b), "{action}");
            for name in a.headers().keys().filter(|n| *n != header::SET_COOKIE) {
                assert_eq!(a.headers().get_all(name), b.headers().get_all(name));
            }
            assert_eq!(a.extensions().get::<u32>(), b.extensions().get::<u32>());
            assert_eq!(
                a.into_body().collect(4096).await.unwrap(),
                b.into_body().collect(4096).await.unwrap(),
                "{action}"
            );
        }
    }
}

#[tokio::test]
async fn missing_layer_fails_with_compatible_status_and_body() {
    let response = Router::new()
        .route("/", post(owned))
        .oneshot(request("read", &[]))
        .await
        .unwrap();
    assert_eq!(response.status(), 500);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/plain; charset=utf-8"
    );
    assert_eq!(
        response.into_body().collect(4096).await.unwrap(),
        "Can't extract cookies. Is `CookieManagerLayer` enabled?"
    );
}

#[test]
fn detached_jar_clones_share_state_and_deltas_are_not_drained() {
    let mut incoming = HeaderMap::new();
    incoming.append(header::COOKIE, HeaderValue::from_static("original=value"));
    let cookies = Cookies::from_headers(&incoming);
    assert!(cookies.delta().is_empty());
    assert_eq!(cookies.get("original").unwrap().value(), "value");
    assert!(cookies.delta().is_empty());
    let clone = cookies.clone();
    clone.add(Cookie::new("new", "value"));
    cookies.remove(Cookie::build(("original", "")).path("/app").build());
    assert!(clone.get("original").is_none());
    assert_eq!(cookies.get("new").unwrap().value(), "value");
    let mut output = HeaderMap::new();
    output.append(
        header::SET_COOKIE,
        HeaderValue::from_static("existing=keep"),
    );
    assert_eq!(cookies.append_delta(&mut output), 2);
    assert_eq!(output.get_all(header::SET_COOKIE).iter().count(), 3);
    assert_eq!(cookies.delta().len(), 2);
    let mut second = HeaderMap::new();
    assert_eq!(clone.append_delta(&mut second), 2);
}

#[tokio::test]
async fn multiple_extractions_and_middleware_share_one_request_but_not_other_requests() {
    async fn before(cookies: Cookies, request: Request, next: Next) -> Response {
        cookies.add(Cookie::new("before", "value"));
        let mut response = next.run(request).await;
        cookies.add(Cookie::new("after", "value"));
        response.headers_mut().append(
            header::SET_COOKIE,
            HeaderValue::from_static("manual=preserved"),
        );
        response
    }
    async fn handler(a: Cookies, b: Cookies) -> String {
        assert!(a.get("handler").is_none());
        assert_eq!(a.get("before").unwrap().value(), "value");
        b.add(Cookie::new("handler", "shared"));
        a.get("handler").unwrap().value().to_owned()
    }
    let router = Router::new()
        .route("/", post(handler))
        .layer(from_fn(before))
        .layer(CookieManagerLayer::new());
    for _ in 0..2 {
        let response = router.clone().oneshot(request("read", &[])).await.unwrap();
        let delta = normalized_deltas(&response);
        assert_eq!(
            delta,
            [
                "after=value",
                "before=value",
                "handler=shared",
                "manual=preserved"
            ]
        );
        assert_eq!(response.into_body().collect(4096).await.unwrap(), "shared");
    }
}

#[tokio::test]
async fn deltas_survive_downstream_extractor_rejection() {
    async fn before(cookies: Cookies, request: Request, next: Next) -> Response {
        cookies.add(Cookie::new("csrf", "new"));
        next.run(request).await
    }
    async fn reject(_: simple_server::web::Json<serde_json::Value>) {}
    let app = Router::new()
        .route("/", post(reject))
        .layer(from_fn(before))
        .layer(CookieManagerLayer::new());
    let response = app.oneshot(request("read", &[])).await.unwrap();
    assert_eq!(response.status(), 415);
    assert_eq!(normalized_deltas(&response), ["csrf=new"]);
}

#[tokio::test]
async fn readiness_errors_and_non_web_bodies_pass_through() {
    use std::{
        future::{Ready, ready},
        task::{Context, Poll},
    };
    #[derive(Clone)]
    struct Inner;
    impl Service<Request<Vec<u8>>> for Inner {
        type Response = Response<Vec<u8>>;
        type Error = &'static str;
        type Future = Ready<Result<Self::Response, Self::Error>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Err("readiness"))
        }
        fn call(&mut self, request: Request<Vec<u8>>) -> Self::Future {
            assert_eq!(request.body(), b"input");
            let jar = request.extensions().get::<Cookies>().unwrap();
            jar.add(Cookie::new("one", "value"));
            if request.uri().path() == "/error" {
                ready(Err("inner error"))
            } else {
                ready(Ok(Response::new(b"stream untouched".to_vec())))
            }
        }
    }
    let mut service = CookieManagerLayer::new().layer(Inner);
    assert!(matches!(service.ready().await, Err("readiness")));
    let response = service
        .call(Request::builder().uri("/").body(b"input".to_vec()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.body(), b"stream untouched");
    assert_eq!(response.headers()[header::SET_COOKIE], "one=value");
    assert_eq!(
        service
            .call(
                Request::builder()
                    .uri("/error")
                    .body(b"input".to_vec())
                    .unwrap()
            )
            .await
            .unwrap_err(),
        "inner error"
    );
}

#[tokio::test]
async fn manually_installed_extension_extracts_without_implicit_output_policy() {
    let mut parts = Request::new(Body::empty()).into_parts().0;
    let jar = Cookies::default();
    parts.extensions.insert(jar.clone());
    let extracted = Cookies::from_request_parts(&mut parts, &()).await.unwrap();
    extracted.add(Cookie::new("manual", "value"));
    assert_eq!(jar.get("manual").unwrap().value(), "value");
}

#[cfg(feature = "test-harness")]
#[tokio::test]
async fn login_read_logout_cookie_contract_over_real_tcp() {
    use simple_server::testing::TestServer;
    async fn login(cookies: Cookies) -> &'static str {
        cookies.add(
            Cookie::build(("__Host-session", "opaque / id"))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Lax)
                .max_age(time::Duration::days(7))
                .build(),
        );
        cookies.add(
            Cookie::build(("__Host-csrf", "token"))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .build(),
        );
        "logged in"
    }
    async fn session(cookies: Cookies) -> String {
        cookies
            .get("__Host-session")
            .map(|c| c.value().to_owned())
            .unwrap_or_default()
    }
    async fn logout(cookies: Cookies) {
        cookies.add(
            Cookie::build(("__Host-session", ""))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Lax)
                .max_age(time::Duration::ZERO)
                .build(),
        );
    }
    let app = Router::new()
        .route("/login", post(login))
        .route("/session", post(session))
        .route("/logout", post(logout))
        .layer(CookieManagerLayer::new());
    let server = TestServer::tcp(app).await.unwrap();
    let response = server.post("/login").send().await.unwrap();
    response.assert_text("logged in");
    assert_eq!(
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .count(),
        2
    );
    let session_cookie = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find(|v| v.to_str().unwrap().starts_with("__Host-session="))
        .unwrap()
        .to_str()
        .unwrap();
    let parsed = Cookie::parse(session_cookie.to_owned()).unwrap();
    assert_eq!(parsed.path(), Some("/"));
    assert_eq!(parsed.domain(), None);
    assert_eq!(parsed.secure(), Some(true));
    assert_eq!(parsed.http_only(), Some(true));
    assert_eq!(parsed.same_site(), Some(SameSite::Lax));
    assert_eq!(parsed.max_age(), Some(time::Duration::days(7)));
    let response = server
        .post("/session")
        .header("cookie", "__Host-session=opaque%20%2F%20id")
        .send()
        .await
        .unwrap();
    response.assert_text("opaque / id");
    assert!(!response.headers().contains_key(header::SET_COOKIE));
    let response = server.post("/logout").send().await.unwrap();
    let removal = Cookie::parse(
        response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    assert_eq!(removal.name(), "__Host-session");
    assert_eq!(removal.value(), "");
    assert_eq!(removal.max_age(), Some(time::Duration::ZERO));
    assert_eq!(removal.path(), Some("/"));
    assert_eq!(removal.secure(), Some(true));
    assert_eq!(removal.http_only(), Some(true));
    server.shutdown().await.unwrap();
}

#[test]
fn concurrent_jar_clones_keep_independent_mutations() {
    let jar = Cookies::default();
    std::thread::scope(|scope| {
        for n in 0..16 {
            let jar = jar.clone();
            scope.spawn(move || jar.add(Cookie::new(format!("cookie-{n}"), n.to_string())));
        }
    });
    assert_eq!(jar.list().len(), 16);
    assert_eq!(jar.delta().len(), 16);
}

#[tokio::test]
async fn response_stream_is_not_polled_or_collected_by_cookie_layer() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let polls = Arc::new(AtomicUsize::new(0));
    let state = polls.clone();
    let inner = tower::service_fn(move |request: Request| {
        let state = state.clone();
        async move {
            request
                .extensions()
                .get::<Cookies>()
                .unwrap()
                .add(Cookie::new("stream", "kept"));
            let mut sent = false;
            let stream = futures_util::stream::poll_fn(move |_| {
                state.fetch_add(1, Ordering::Relaxed);
                std::task::Poll::Ready(if sent {
                    None
                } else {
                    sent = true;
                    Some(Ok::<_, std::io::Error>(b"streamed data".to_vec()))
                })
            });
            Ok::<_, std::convert::Infallible>(Response::new(Body::from_stream(stream)))
        }
    });
    let response = CookieManagerLayer::new()
        .layer(inner)
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap();
    assert_eq!(polls.load(Ordering::Relaxed), 0);
    assert_eq!(response.headers()[header::SET_COOKIE], "stream=kept");
    assert_eq!(
        response.into_body().collect(4096).await.unwrap(),
        "streamed data"
    );
    assert!(polls.load(Ordering::Relaxed) > 0);
}

#[tokio::test]
async fn cancelling_pending_response_releases_the_inner_future() {
    use std::{
        future::Future,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    let state = dropped.clone();
    let inner = tower::service_fn(move |request: Request| {
        let guard = Guard(state.clone());
        async move {
            let _guard = guard;
            request
                .extensions()
                .get::<Cookies>()
                .unwrap()
                .add(Cookie::new("pending", "mutation"));
            std::future::pending::<Result<Response, std::convert::Infallible>>().await
        }
    });
    let mut future = Box::pin(
        CookieManagerLayer::new()
            .layer(inner)
            .oneshot(Request::new(Body::empty())),
    );
    let waker = futures_util::task::noop_waker();
    assert!(
        future
            .as_mut()
            .poll(&mut std::task::Context::from_waker(&waker))
            .is_pending()
    );
    assert_eq!(dropped.load(Ordering::Relaxed), 0);
    drop(future);
    assert_eq!(dropped.load(Ordering::Relaxed), 1);
}
