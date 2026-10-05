#![cfg(feature = "engine-web")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use simple_server::engine_web::IntoResponse;
use simple_server::engine_web::{self as e, FromRequest, FromRequestParts, Handler};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const MIME: &str = "application/x-www-form-urlencoded";
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Input {
    count: u32,
    label: String,
}
fn request(method: &str, uri: &str, body: impl Into<e::Body>, content: Option<&str>) -> e::Request {
    let mut request = http::Request::builder().method(method).uri(uri);
    if let Some(content) = content {
        request = request.header("content-type", content);
    }
    request.body(body.into()).unwrap()
}
async fn parts(response: e::Response) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect(4 * 1024 * 1024).await.unwrap().to_vec(),
    )
}
fn counted_body(polls: Arc<AtomicUsize>) -> e::Body {
    e::Body::from_stream(futures_util::stream::poll_fn(move |_| {
        polls.fetch_add(1, Ordering::SeqCst);
        std::task::Poll::Ready(Some(Err::<e::Bytes, _>(std::io::Error::other(
            "read failed",
        ))))
    }))
}
#[simple_server::test]
async fn form_method_selection_and_decoding_are_explicit() {
    for method in ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"] {
        let value = e::Form::<Input>::from_request(
            request(
                method,
                "/?count=3&label=query",
                "count=7&label=a+b%2Bc",
                Some(MIME),
            ),
            &(),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(
            value,
            if method == "GET" {
                Input {
                    count: 3,
                    label: "query".into(),
                }
            } else {
                Input {
                    count: 7,
                    label: "a b+c".into(),
                }
            }
        );
    }
    for (method, expected) in [("GET", 400), ("HEAD", 400), ("POST", 422)] {
        let error = e::Form::<Input>::from_request(
            request(method, "/?count=bad", "count=bad", Some(MIME)),
            &(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.status().as_u16(), expected);
    }
}
#[simple_server::test]
async fn form_body_limits_errors_and_short_circuiting_are_preserved() {
    let polls = Arc::new(AtomicUsize::new(0));
    let value = e::Form::<Input>::from_request(
        request(
            "GET",
            "/?count=1&label=query",
            counted_body(polls.clone()),
            None,
        ),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(value.0.count, 1);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let error = e::Form::<Input>::from_request(
        request(
            "POST",
            "/",
            counted_body(polls.clone()),
            Some("application/json"),
        ),
        &(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.status(), e::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let error = e::Form::<Input>::from_request(
        request("POST", "/", counted_body(polls.clone()), Some(MIME)),
        &(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.status(), e::StatusCode::BAD_REQUEST);
    assert!(polls.load(Ordering::SeqCst) > 0);
    let prefix = "count=1&label=";
    let exact = format!("{prefix}{}", "a".repeat(2 * 1024 * 1024 - prefix.len()));
    assert!(
        e::Form::<Input>::from_request(request("POST", "/", exact.clone(), Some(MIME)), &())
            .await
            .is_ok()
    );
    assert_eq!(
        e::Form::<Input>::from_request(request("POST", "/", format!("{exact}a"), Some(MIME)), &())
            .await
            .unwrap_err()
            .status(),
        e::StatusCode::PAYLOAD_TOO_LARGE
    );
    let mut limited = request("POST", "/", "count=1&label=x", Some(MIME));
    limited.extensions_mut().insert(e::BodyLimit(2));
    assert_eq!(
        e::Form::<Input>::from_request(limited, &())
            .await
            .unwrap_err()
            .status(),
        e::StatusCode::PAYLOAD_TOO_LARGE
    );
}
#[derive(Clone, Debug, PartialEq)]
struct Local(String);
#[simple_server::test]
async fn host_extensions_are_typed_optional_and_short_circuit_body_reads() {
    let polls = Arc::new(AtomicUsize::new(0));
    async fn handler(
        e::Extension(local): e::Extension<Local>,
        e::Form(input): e::Form<Input>,
    ) -> String {
        format!("{}:{}", local.0, input.count)
    }
    let response = handler
        .call(
            request("POST", "/", counted_body(polls.clone()), Some(MIME)),
            (),
        )
        .await;
    assert_eq!(response.status(), e::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let (mut head, _) = request("GET", "/", "", None).into_parts();
    assert!(
        Option::<e::Extension<Local>>::from_request_parts(&mut head, &())
            .await
            .unwrap()
            .is_none()
    );
    head.extensions.insert(Local("local".into()));
    head.extensions.insert(42u32);
    assert_eq!(
        e::Extension::<Local>::from_request_parts(&mut head, &())
            .await
            .unwrap()
            .0,
        Local("local".into())
    );
    assert_eq!(
        Option::<e::Extension<Local>>::from_request_parts(&mut head, &())
            .await
            .unwrap()
            .unwrap()
            .0,
        Local("local".into())
    );
    assert_eq!(head.extensions.get::<u32>(), Some(&42));
    let mut req = request("POST", "/", "count=7&label=x", Some(MIME));
    req.extensions_mut().insert(Local("host".into()));
    assert_eq!(parts(handler.call(req, ()).await).await.2, b"host:7");
}
#[cfg(feature = "web")]
async fn source_parts(
    response: simple_server::web::Response,
) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect(4 * 1024 * 1024).await.unwrap().to_vec(),
    )
}
#[cfg(feature = "web")]
#[simple_server::test]
async fn form_extraction_matches_source_status_headers_and_body() {
    use simple_server::web::{self as w, FromRequest as _, IntoResponse as _};
    for method in ["GET", "HEAD", "POST", "PUT"] {
        for content in [
            None,
            Some(MIME),
            Some("application/x-www-form-urlencoded; charset=utf-8"),
            Some("application/x-www-form-urlencoded-extra"),
            Some("Application/X-Www-Form-Urlencoded"),
            Some("application/json"),
        ] {
            for (uri, body) in [
                ("/?count=3&label=a+b%2Bc", "count=7&label=body"),
                ("/?count=bad&label=x", "count=bad&label=x"),
                ("/", ""),
                ("/?count=1&count=2&label=x", "count=1&count=2&label=x"),
                ("/?count=1&label=%FF", "count=1&label=%FF"),
            ] {
                let er = request(method, uri, body, content);
                let (head, _) = request(method, uri, "", content).into_parts();
                let sr = w::Request::from_parts(head, w::Body::from(body));
                let engine = match e::Form::<Input>::from_request(er, &()).await {
                    Ok(value) => e::Json(value.0).into_response(),
                    Err(error) => e::IntoResponse::into_response(error),
                };
                let source = match w::Form::<Input>::from_request(sr, &()).await {
                    Ok(value) => w::Json(value.0).into_response(),
                    Err(error) => w::IntoResponse::into_response(error),
                };
                assert_eq!(
                    parts(engine).await,
                    source_parts(source).await,
                    "{method} {content:?} {uri} {body}"
                );
            }
        }
    }
}
#[cfg(feature = "web")]
#[simple_server::test]
async fn extension_rejections_and_form_responses_match_source() {
    use simple_server::web::{self as w, IntoResponse as _};
    let (mut head, _) = request("GET", "/", "", None).into_parts();
    let engine = e::Extension::<Local>::from_request_parts(&mut head, &())
        .await
        .unwrap_err();
    let source = w::Extension::<Local>::from_request_parts(&mut head, &())
        .await
        .unwrap_err();
    assert_eq!(
        parts(e::IntoResponse::into_response(engine)).await,
        source_parts(w::IntoResponse::into_response(source)).await
    );
    assert_eq!(
        parts(
            e::Form(Input {
                count: 7,
                label: "a b+c".into()
            })
            .into_response()
        )
        .await,
        source_parts(
            w::Form(Input {
                count: 7,
                label: "a b+c".into()
            })
            .into_response()
        )
        .await
    );
    // Unsupported top-level values fail serialization instead of emitting a partial form.
    assert_eq!(
        parts(e::Form(7u32).into_response()).await,
        source_parts(w::Form(7u32).into_response()).await
    );
}
