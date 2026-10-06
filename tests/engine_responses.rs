#![cfg(feature = "engine-web")]
use simple_server::engine_web::{self as e, IntoResponse, response::Redirect};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
async fn snapshot(response: e::Response) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
    let (head, body) = response.into_parts();
    (
        head.status,
        head.headers,
        body.collect(1024).await.unwrap().to_vec(),
    )
}
#[simple_server::test]
async fn arrays_replace_values_preserve_head_and_support_empty_responses() {
    let mut response = e::Response::new(e::Body::from("body"));
    *response.status_mut() = e::StatusCode::ACCEPTED;
    *response.version_mut() = e::Version::HTTP_2;
    response.extensions_mut().insert(42u32);
    response
        .headers_mut()
        .append("set-cookie", "a=1".parse().unwrap());
    response
        .headers_mut()
        .append("set-cookie", "b=2".parse().unwrap());
    let response = ([("set-cookie", "c=3"), ("set-cookie", "d=4")], response).into_response();
    assert_eq!(response.status(), e::StatusCode::ACCEPTED);
    assert_eq!(response.version(), e::Version::HTTP_2);
    assert_eq!(response.extensions().get::<u32>(), Some(&42));
    assert_eq!(response.headers().get_all("set-cookie").iter().count(), 1);
    assert_eq!(response.headers()["set-cookie"], "d=4");
    assert_eq!(snapshot(response).await.2, b"body");
    let response = [("x-empty", "yes")].into_response();
    assert_eq!(response.headers()["x-empty"], "yes");
    assert!(snapshot(response).await.2.is_empty());
}
struct ObservedBody {
    polled: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
}
impl Drop for ObservedBody {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
impl futures_util::Stream for ObservedBody {
    type Item = Result<e::Bytes, std::io::Error>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.polled.store(true, Ordering::SeqCst);
        std::task::Poll::Ready(None)
    }
}
#[simple_server::test]
async fn invalid_headers_discard_partial_response_without_polling_its_body() {
    let polled = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let body = e::Body::from_stream(ObservedBody {
        polled: polled.clone(),
        dropped: dropped.clone(),
    });
    let response = (
        e::StatusCode::CREATED,
        [("x-first", "valid"), ("bad header", "invalid")],
        body,
    )
        .into_response();
    assert_eq!(response.status(), e::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.headers().contains_key("x-first"));
    assert!(!polled.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(snapshot(response).await.2, b"invalid HTTP header name");
    assert_eq!(
        (e::StatusCode::CREATED, [("x-test", "bad\nvalue")], "body")
            .into_response()
            .status(),
        e::StatusCode::INTERNAL_SERVER_ERROR
    );
    // An outer status wrapper is a separate conversion and deliberately overrides the inner error.
    assert_eq!(
        (e::StatusCode::CREATED, ([("x-test", "bad\nvalue")], "body"))
            .into_response()
            .status(),
        e::StatusCode::CREATED
    );
}
#[simple_server::test]
async fn redirects_expose_location_and_reject_invalid_header_values_during_conversion() {
    for (make, status) in [
        (Redirect::to as fn(&str) -> Redirect, 303),
        (Redirect::temporary, 307),
        (Redirect::permanent, 308),
    ] {
        let redirect = make("/next?x=1");
        assert_eq!(redirect.status_code().as_u16(), status);
        assert_eq!(redirect.clone().location(), "/next?x=1");
        let (actual, headers, body) = snapshot(redirect.into_response()).await;
        assert_eq!(actual.as_u16(), status);
        assert_eq!(headers["location"], "/next?x=1");
        assert!(body.is_empty());
        let invalid = make("/next\r\nx-injected: yes");
        assert_eq!(invalid.status_code().as_u16(), status);
        let (actual, headers, body) = snapshot(invalid.into_response()).await;
        assert_eq!(actual, e::StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!headers.contains_key("location"));
        assert_eq!(body, b"failed to parse header value");
    }
}
#[cfg(feature = "web")]
mod parity {
    use super::*;
    use simple_server::web as s;
    async fn source(response: s::Response) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
        let (head, body) = response.into_parts();
        (
            head.status,
            head.headers,
            body.collect(1024).await.unwrap().to_vec(),
        )
    }
    async fn compare<T: e::IntoResponse + s::IntoResponse + Clone>(value: T) {
        assert_eq!(
            snapshot(e::IntoResponse::into_response(value.clone())).await,
            source(s::IntoResponse::into_response(value)).await
        );
    }
    #[simple_server::test]
    async fn header_arrays_match_source_including_conversion_errors_and_status_precedence() {
        compare([("x-test", "value")]).await;
        compare([] as [(&str, &str); 0]).await;
        compare((
            [("content-type", "first"), ("content-type", "second")],
            "body",
        ))
        .await;
        compare((e::StatusCode::CREATED, [("x-test", "ok")], "body")).await;
        compare([(
            http::header::SET_COOKIE,
            e::HeaderValue::from_bytes(&[0x80, 0xff]).unwrap(),
        )])
        .await;
        for (name, value) in [
            ("bad name", "valid"),
            ("x-test", "bad\nvalue"),
            ("bad name", "bad\nvalue"),
            ("", "valid"),
        ] {
            let headers = [("x-first", "valid"), (name, value)];
            compare(headers).await;
            compare((headers, "original body")).await;
            compare((e::StatusCode::CREATED, headers, "original body")).await;
            compare((e::StatusCode::CREATED, (headers, "original body"))).await;
        }
        let mut headers = e::HeaderMap::new();
        headers.append("set-cookie", "a=1".parse().unwrap());
        headers.append("set-cookie", "b=2".parse().unwrap());
        compare(([("set-cookie", "c=3")], (headers, "body"))).await;
    }
    #[simple_server::test]
    async fn redirects_match_source_for_relative_absolute_unicode_and_invalid_locations() {
        for uri in [
            "/next",
            "https://example.test/a?b=c",
            "",
            "/café",
            "bad\nvalue",
            "bad\0value",
        ] {
            for (engine, source_redirect) in [
                (Redirect::to(uri), s::response::Redirect::to(uri)),
                (
                    Redirect::temporary(uri),
                    s::response::Redirect::temporary(uri),
                ),
                (
                    Redirect::permanent(uri),
                    s::response::Redirect::permanent(uri),
                ),
            ] {
                assert_eq!(engine.status_code(), source_redirect.status_code());
                assert_eq!(engine.location(), source_redirect.location());
                assert_eq!(
                    snapshot(engine.into_response()).await,
                    source(s::IntoResponse::into_response(source_redirect)).await
                );
            }
        }
    }
}
