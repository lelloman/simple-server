#![cfg(all(feature = "web", feature = "tower-cookies", feature = "body-limit"))]
use simple_server::web::{self, Body, Form, IntoResponse, Request, Router};
use tower::ServiceExt;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Input {
    name: String,
    count: u32,
}

#[tokio::test]
async fn form_extraction_matches_query_body_and_rejection_contracts() {
    let shared = Router::new()
        .route(
            "/",
            web::routing::any(|Form(value): Form<Input>| async { web::Json(value) }),
        )
        .layer(simple_server::body_limit::BodyLimit::max(32));
    let old = axum::Router::new()
        .route(
            "/",
            axum::routing::any(|axum::Form(value): axum::Form<Input>| async { axum::Json(value) }),
        )
        .layer(simple_server::body_limit::BodyLimit::max(32));
    for (method, uri, content_type, body) in [
        ("GET", "/?name=hello+world&count=3", None, "ignored"),
        ("HEAD", "/?name=hello&count=3", None, "ignored"),
        ("GET", "/?name=a&count=bad", None, ""),
        (
            "POST",
            "/",
            Some("application/x-www-form-urlencoded"),
            "name=hello%2Bworld&count=3",
        ),
        (
            "POST",
            "/",
            Some("application/x-www-form-urlencoded; charset=utf-8"),
            "name=hello&count=3",
        ),
        ("POST", "/", None, "name=hello&count=3"),
        ("POST", "/", Some("application/json"), "{}"),
        (
            "POST",
            "/",
            Some("application/x-www-form-urlencoded"),
            "name=a&count=bad",
        ),
        (
            "POST",
            "/",
            Some("application/x-www-form-urlencoded"),
            "name=a&name=b&count=3",
        ),
        (
            "POST",
            "/",
            Some("application/x-www-form-urlencoded"),
            "name=abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz&count=3",
        ),
    ] {
        let make = || {
            let mut builder = Request::builder().method(method).uri(uri);
            if let Some(value) = content_type {
                builder = builder.header("content-type", value);
            }
            builder.body(Body::from(body)).unwrap()
        };
        let actual = shared.clone().oneshot(make()).await.unwrap();
        let expected = old.clone().oneshot(make()).await.unwrap();
        assert_eq!(actual.status(), expected.status(), "{method} {body}");
        assert_eq!(actual.headers(), expected.headers());
        assert_eq!(
            actual.into_body().collect(8192).await.unwrap(),
            axum::body::to_bytes(expected.into_body(), 8192)
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
async fn html_redirect_and_form_responses_preserve_wire_bytes() {
    let mut pairs = vec![(
        web::response::Html("<h1>Hi & bye</h1>").into_response(),
        axum::response::IntoResponse::into_response(axum::response::Html("<h1>Hi & bye</h1>")),
    )];
    for uri in [
        "/next?state=a%2Bb",
        "https://example.test/path",
        "/invalid\nheader",
    ] {
        pairs.push((
            web::response::Redirect::to(uri).into_response(),
            axum::response::IntoResponse::into_response(axum::response::Redirect::to(uri)),
        ));
        pairs.push((
            web::response::Redirect::temporary(uri).into_response(),
            axum::response::IntoResponse::into_response(axum::response::Redirect::temporary(uri)),
        ));
        pairs.push((
            web::response::Redirect::permanent(uri).into_response(),
            axum::response::IntoResponse::into_response(axum::response::Redirect::permanent(uri)),
        ));
    }
    pairs.push((
        Form(Input {
            name: "a & b".into(),
            count: 3,
        })
        .into_response(),
        axum::response::IntoResponse::into_response(axum::Form(Input {
            name: "a & b".into(),
            count: 3,
        })),
    ));
    pairs.push((
        (web::StatusCode::FOUND, [("location", "/oauth")]).into_response(),
        axum::response::IntoResponse::into_response((
            web::StatusCode::FOUND,
            [("location", "/oauth")],
        )),
    ));
    for (actual, expected) in pairs {
        assert_eq!(actual.status(), expected.status());
        assert_eq!(actual.headers(), expected.headers());
        assert_eq!(
            actual.into_body().collect(8192).await.unwrap(),
            axum::body::to_bytes(expected.into_body(), 8192)
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
async fn cookies_preserve_jar_deltas_and_missing_layer_errors() {
    use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
    async fn handler(cookies: Cookies) -> String {
        let old = cookies
            .get("session")
            .map(|value| value.value().to_owned())
            .unwrap_or_default();
        cookies.add(
            Cookie::build(("session", "new"))
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(tower_cookies::cookie::SameSite::Strict)
                .build(),
        );
        cookies.add(Cookie::new("csrf", "token"));
        cookies.remove(Cookie::from("obsolete"));
        old
    }
    let app = Router::new().route("/", web::routing::get(handler));
    let request = || {
        Request::builder()
            .uri("/")
            .header("cookie", "session=previous; obsolete=old")
            .body(Body::empty())
            .unwrap()
    };
    let missing = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(missing.status(), 500);
    assert_eq!(
        missing.headers()["content-type"],
        "text/plain; charset=utf-8"
    );
    assert_eq!(
        missing.into_body().collect(1024).await.unwrap(),
        "Can't extract cookies. Is `CookieManagerLayer` enabled?"
    );
    let response = app
        .layer(CookieManagerLayer::new())
        .oneshot(request())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let values: Vec<_> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect();
    assert_eq!(values.len(), 3);
    let session = values
        .iter()
        .find(|value| value.starts_with("session="))
        .unwrap();
    for attr in [
        "session=new",
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/",
    ] {
        assert!(session.contains(attr), "{session}");
    }
    assert!(
        values
            .iter()
            .any(|value| value.starts_with("obsolete=") && value.contains("Max-Age=0"))
    );
    assert_eq!(
        response.into_body().collect(1024).await.unwrap(),
        "previous"
    );
}

#[tokio::test]
async fn response_mapper_runs_once_for_success_and_extraction_failure() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let app = Router::new()
        .route(
            "/",
            web::routing::post(|Form(value): Form<Input>| async { value.name }),
        )
        .layer(web::middleware::map_response(
            move |mut response: web::Response| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    response
                        .headers_mut()
                        .insert("cache-control", "no-store".parse().unwrap());
                    response
                }
            },
        ));
    for (body, status) in [("name=good&count=3", 200), ("bad", 422)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn method_layer_scopes_form_body_limits_to_selected_routes() {
    async fn form(Form(value): Form<Input>) -> String {
        value.name
    }
    let app = Router::new()
        .route(
            "/limited",
            web::routing::post(form).layer(simple_server::body_limit::BodyLimit::max(8)),
        )
        .route("/ordinary", web::routing::post(form));
    for (path, status) in [("/limited", 413), ("/ordinary", 200)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("name=example&count=3"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
}
