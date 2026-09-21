#![cfg(feature = "response-headers")]

use simple_server::response_headers::{
    HeaderMap, HeaderValue, header, insert_if_absent, merge_vary, replace,
};

fn values(headers: &HeaderMap, name: http::HeaderName) -> Vec<Vec<u8>> {
    headers
        .get_all(name)
        .iter()
        .map(|v| v.as_bytes().to_vec())
        .collect()
}

#[test]
fn defaults_preserve_all_values_and_replacement_removes_only_the_selected_field() {
    let mut headers = HeaderMap::new();
    let mut secret = HeaderValue::from_static("session=a; Secure");
    secret.set_sensitive(true);
    headers.append(header::SET_COOKIE, secret.clone());
    headers.append(header::SET_COOKIE, HeaderValue::from_static("other=b"));
    headers.append(header::CACHE_CONTROL, HeaderValue::from_static(""));
    headers.append(header::CACHE_CONTROL, HeaderValue::from_static("private"));
    let before = headers.clone();
    insert_if_absent(
        &mut headers,
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    insert_if_absent(
        &mut headers,
        header::SET_COOKIE,
        HeaderValue::from_static("new=c"),
    );
    assert_eq!(headers, before);
    assert!(headers[header::SET_COOKIE].is_sensitive());
    replace(
        &mut headers,
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    assert_eq!(
        values(&headers, header::CACHE_CONTROL),
        [b"no-store".to_vec()]
    );
    assert_eq!(
        values(&headers, header::SET_COOKIE),
        values(&before, header::SET_COOKIE)
    );
    assert!(headers[header::SET_COOKIE].is_sensitive());
    insert_if_absent(
        &mut headers,
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'"),
    );
    assert_eq!(
        headers[header::CONTENT_SECURITY_POLICY],
        "default-src 'none'"
    );
}

#[test]
fn vary_preserves_lines_bytes_duplicates_and_flags_while_adding_missing_names() {
    let mut headers = HeaderMap::new();
    let mut original = HeaderValue::from_static("Accept-Encoding, COOKIE, cookie");
    original.set_sensitive(true);
    headers.append(header::VARY, original);
    headers.append(
        header::VARY,
        HeaderValue::from_static(" \tAuthorization\t, "),
    );
    headers.append(header::VARY, HeaderValue::from_bytes(b"\xff").unwrap());
    let before = headers.clone();
    merge_vary(&mut headers, "Cookie").unwrap();
    merge_vary(&mut headers, "authorization").unwrap();
    assert_eq!(headers, before);
    merge_vary(&mut headers, "Origin").unwrap();
    let mut expected = values(&before, header::VARY);
    expected.push(b"Origin".to_vec());
    assert_eq!(values(&headers, header::VARY), expected);
    assert!(headers[header::VARY].is_sensitive());
    merge_vary(&mut headers, "ORIGIN").unwrap();
    assert_eq!(values(&headers, header::VARY), expected);
}

#[test]
fn wildcard_dominates_across_all_lines_and_can_be_explicitly_requested() {
    for value in ["*", "Accept, *", " \t*\t "] {
        let mut headers = HeaderMap::new();
        headers.append(header::VARY, HeaderValue::from_static("Cookie"));
        headers.append(header::VARY, HeaderValue::from_str(value).unwrap());
        let before = headers.clone();
        merge_vary(&mut headers, "Origin").unwrap();
        merge_vary(&mut headers, "*").unwrap();
        assert_eq!(headers, before);
    }
    let mut headers = HeaderMap::new();
    merge_vary(&mut headers, "Cookie").unwrap();
    merge_vary(&mut headers, "*").unwrap();
    assert_eq!(
        values(&headers, header::VARY),
        [b"Cookie".to_vec(), b"*".to_vec()]
    );
    let before = headers.clone();
    merge_vary(&mut headers, "Origin").unwrap();
    assert_eq!(headers, before);
}

#[test]
fn invalid_vary_input_is_rejected_without_mutation_even_with_an_existing_wildcard() {
    for required in [
        "",
        "Cookie, Authorization",
        " Cookie",
        "Cookie ",
        "a:b",
        "\r\ninjected: yes",
        "é",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::VARY, HeaderValue::from_static("*"));
        let before = headers.clone();
        assert!(merge_vary(&mut headers, required).is_err(), "{required:?}");
        assert_eq!(headers, before);
    }
}

#[cfg(all(feature = "http", feature = "http-tracing"))]
#[tokio::test]
async fn response_status_extensions_and_lazy_data_trailers_errors_survive() {
    use futures_util::StreamExt;
    use http_body_util::{BodyExt, StreamBody};
    use simple_server::axum::body::{Body, Bytes};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    // Use Axum's re-export only in the integration test; the module has no Axum dependency.
    // Frame comes from http-body, enabled here by the all-features test run.
    let polls = Arc::new(AtomicUsize::new(0));
    let mut trailers = HeaderMap::new();
    trailers.insert("x-trailer", HeaderValue::from_static("complete"));
    let frames = vec![
        Ok(http_body::Frame::data(Bytes::from_static(
            b"data: event\n\n",
        ))),
        Ok(http_body::Frame::trailers(trailers.clone())),
        Err(std::io::Error::other("stream failure")),
    ];
    let counter = polls.clone();
    let stream = futures_util::stream::iter(frames).inspect(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    let body = Body::new(StreamBody::new(stream));
    let mut response = http::Response::builder()
        .status(206)
        .version(http::Version::HTTP_2)
        .body(body)
        .unwrap();
    response.extensions_mut().insert(123u32);
    insert_if_absent(
        response.headers_mut(),
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    merge_vary(response.headers_mut(), "Authorization").unwrap();
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert_eq!(response.status(), 206);
    assert_eq!(response.version(), http::Version::HTTP_2);
    assert_eq!(response.extensions().get::<u32>(), Some(&123));
    let body = response.body_mut();
    assert_eq!(
        body.frame().await.unwrap().unwrap().into_data().unwrap(),
        "data: event\n\n"
    );
    assert_eq!(
        body.frame()
            .await
            .unwrap()
            .unwrap()
            .into_trailers()
            .unwrap(),
        trailers
    );
    assert!(body.frame().await.unwrap().is_err());
    assert!(body.frame().await.is_none());
    assert_eq!(polls.load(Ordering::SeqCst), 3);
}
