#![cfg(feature = "engine-web")]
use serde::{Deserialize, Serialize};
use simple_server::engine_web::{self as e, FromRequestParts, Handler};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn request(params: &[(&str, &str)]) -> e::Request {
    let mut request = e::Request::new(e::Body::empty());
    request.extensions_mut().insert(e::RequestMetadata {
        peer: None,
        matched_path: Some("/test".into()),
        original_uri: None,
        path_params: params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        path_error: None,
    });
    request
}
#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Params {
    id: u32,
    name: String,
}
#[simple_server::test]
async fn path_uses_owned_decoded_metadata_and_can_be_extracted_twice() {
    let (mut parts, _) = request(&[("id", "7"), ("name", "a+b/%2F")]).into_parts();
    assert_eq!(
        e::Path::<Params>::from_request_parts(&mut parts, &())
            .await
            .unwrap()
            .0,
        Params {
            id: 7,
            name: "a+b/%2F".into()
        }
    );
    assert_eq!(
        e::Path::<(u32, String)>::from_request_parts(&mut parts, &())
            .await
            .unwrap()
            .0,
        (7, "a+b/%2F".into())
    );
    assert_eq!(
        e::Path::<Vec<(String, String)>>::from_request_parts(&mut parts, &())
            .await
            .unwrap()
            .0,
        vec![("id".into(), "7".into()), ("name".into(), "a+b/%2F".into())]
    );
}
#[simple_server::test]
async fn missing_metadata_and_bad_path_reject_before_body_polling() {
    let (mut missing, _) = e::Request::new(e::Body::empty()).into_parts();
    let error = e::Path::<String>::from_request_parts(&mut missing, &())
        .await
        .unwrap_err();
    assert_eq!(error.status(), e::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error.body(), b"No paths parameters found for matched route");
    let polls = Arc::new(AtomicUsize::new(0));
    let observed = polls.clone();
    let body = e::Body::from_stream(futures_util::stream::poll_fn(move |_| {
        observed.fetch_add(1, Ordering::SeqCst);
        std::task::Poll::Ready(None::<Result<e::Bytes, std::io::Error>>)
    }));
    let mut req = request(&[("id", "bad")]);
    *req.body_mut() = body;
    async fn handler(_: e::Path<u32>, _: String) -> &'static str {
        panic!("invalid path reached handler")
    }
    let response = handler.call(req, ()).await;
    assert_eq!(response.status(), e::StatusCode::BAD_REQUEST);
    assert_eq!(
        response.into_body().collect(1024).await.unwrap(),
        "Invalid URL: Cannot parse `bad` to a `u32`"
    );
    assert_eq!(polls.load(Ordering::SeqCst), 0);
}
#[simple_server::test]
async fn path_result_capture_retains_invalid_utf8_and_configuration_errors() {
    let mut req = request(&[]);
    req.extensions_mut()
        .get_mut::<e::RequestMetadata>()
        .unwrap()
        .path_error = Some((e::StatusCode::BAD_REQUEST, "Invalid UTF-8 in `id`".into()));
    let (mut head, _) = req.into_parts();
    let error = Result::<e::Path<String>, _>::from_request_parts(&mut head, &())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.status(), e::StatusCode::BAD_REQUEST);
    assert_eq!(error.body(), b"Invalid URL: Invalid UTF-8 in `id`");
    let (mut head, _) = request(&[("id", "1"), ("name", "2")]).into_parts();
    assert_eq!(
        e::Path::<u32>::from_request_parts(&mut head, &())
            .await
            .unwrap_err()
            .status(),
        e::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[cfg(all(feature = "web", feature = "client"))]
mod parity {
    use super::*;
    use simple_server::{client::Client, runtime::spawn, time::timeout, web as w};
    use std::{collections::BTreeMap, time::Duration};
    use tower::ServiceExt;
    async fn engine<T: serde::de::DeserializeOwned + Serialize + Send>(
        e::Path(value): e::Path<T>,
    ) -> e::Json<T> {
        e::Json(value)
    }
    async fn source<T: serde::de::DeserializeOwned + Serialize + Send>(
        w::Path(value): w::Path<T>,
    ) -> w::Json<T> {
        w::Json(value)
    }
    async fn compare<T: serde::de::DeserializeOwned + Serialize + Send + 'static>(
        route: &str,
        uris: &[&str],
    ) {
        compare_nested::<T>("/api", route, uris).await;
    }
    async fn compare_nested<T: serde::de::DeserializeOwned + Serialize + Send + 'static>(
        prefix: &str,
        route: &str,
        uris: &[&str],
    ) {
        let inner = e::Router::new()
            .unwrap()
            .route(
                route,
                e::MethodRouter::new()
                    .unwrap()
                    .on_handler(e::Method::GET, engine::<T>)
                    .unwrap(),
            )
            .unwrap();
        let er = e::Router::new().unwrap().nest(prefix, inner).unwrap();
        let sr = w::Router::new().nest(
            prefix,
            w::Router::new().route(route, w::routing::get(source::<T>)),
        );
        let listener = e::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr();
        let stop = e::Shutdown::new();
        let server = spawn(e::serve(listener, er, stop.clone()));
        let client = Client::builder().no_proxy().no_redirect().build().unwrap();
        for uri in uris {
            let uri = format!("/api{uri}");
            let response = timeout(
                Duration::from_secs(5),
                client.get(format!("http://{address}{uri}")).send(),
            )
            .await
            .unwrap()
            .unwrap();
            let status = response.status();
            let mut headers = response.headers().clone();
            // Date is generated by the socket transport, not the extractor.
            headers.remove("date");
            let bytes = response.bytes().await.unwrap().to_vec();
            let expected = sr
                .clone()
                .oneshot(
                    w::Request::builder()
                        .uri(&uri)
                        .body(w::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let (parts, body) = expected.into_parts();
            assert_eq!(
                (status, headers, bytes),
                (
                    parts.status,
                    parts.headers,
                    body.collect(1024 * 1024).await.unwrap().to_vec()
                ),
                "{} {uri}",
                std::any::type_name::<T>()
            );
        }
        stop.request();
        timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    #[derive(Deserialize, Serialize)]
    struct NewId(u32);
    #[derive(Deserialize, Serialize)]
    struct Pair(u32, bool);
    #[derive(Deserialize, Serialize)]
    enum Mode {
        Fast,
        Slow,
    }
    #[derive(Deserialize, Serialize)]
    struct Optional {
        id: u32,
        name: Option<String>,
    }
    #[derive(Deserialize, Serialize)]
    struct Nested {
        id: Vec<u32>,
    }
    #[derive(Deserialize, Serialize)]
    enum Unsupported {
        Value(u32),
        Tuple(u32, u32),
        Fields { id: u32 },
    }
    #[derive(Serialize)]
    struct Custom(String);
    impl<'de> Deserialize<'de> for Custom {
        fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Custom;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("the word allowed")
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Custom, E> {
                    if v == "allowed" {
                        Ok(Custom(v.into()))
                    } else {
                        Err(E::custom("not allowed"))
                    }
                }
            }
            de.deserialize_str(Visitor)
        }
    }
    #[simple_server::test]
    async fn scalars_newtypes_and_custom_errors_match_source_over_http() {
        compare::<u32>("/{id}", &["/42", "/bad", "/-1", "/4294967296", "/%FF"]).await;
        compare::<bool>("/{id}", &["/true", "/false", "/1"]).await;
        compare::<char>("/{id}", &["/a", "/%C3%A9", "/ab"]).await;
        compare::<String>(
            "/{id}",
            &["/a+b", "/a%20b", "/%2F", "/%252F", "/%00", "/%GG", "/%FF"],
        )
        .await;
        compare::<NewId>("/{id}", &["/7", "/bad"]).await;
        compare::<Custom>("/{id}", &["/allowed", "/denied"]).await;
        compare::<(Custom,)>("/{id}", &["/allowed", "/denied"]).await;
        compare::<BTreeMap<String, Custom>>("/{id}", &["/allowed", "/denied"]).await;
    }
    #[simple_server::test]
    async fn tuples_structs_maps_sequences_and_enums_match_source_over_http() {
        compare::<(u32, bool)>("/{id}/{flag}", &["/7/true", "/bad/true", "/7/bad"]).await;
        compare::<Pair>("/{id}/{flag}", &["/7/false", "/bad/true"]).await;
        compare::<Params>("/{id}/{name}", &["/7/alice", "/bad/alice", "/7/%FF"]).await;
        compare::<Optional>("/{id}", &["/7", "/bad"]).await;
        compare::<Optional>("/{id}/{name}", &["/7/alice"]).await;
        compare::<Params>("/{id}/{name}/{extra}", &["/7/alice/ignored"]).await;
        compare::<BTreeMap<String, u32>>("/{id}/{other}", &["/7/8", "/7/bad"]).await;
        compare::<Vec<u32>>("/{id}/{other}", &["/7/8", "/7/bad"]).await;
        compare::<Vec<(String, String)>>("/{id}/{name}", &["/7/alice"]).await;
        compare::<Mode>("/{mode}", &["/Fast", "/Slow", "/unknown"]).await;
        compare::<String>("/{*rest}", &["/some/nested/path", "/a%2Fb/c"]).await;
    }
    #[simple_server::test]
    async fn dynamic_nested_captures_preserve_order_and_duplicate_names() {
        compare_nested::<(String, u32)>("/api/{org}", "/{id}", &["/team/7", "/team/bad"]).await;
        compare_nested::<Vec<(String, String)>>("/api/{id}", "/{id}", &["/outer/inner"]).await;
        compare_nested::<BTreeMap<String, String>>("/api/{id}", "/{id}", &["/outer/inner"]).await;
    }
    #[simple_server::test]
    async fn wrong_counts_missing_fields_and_unsupported_shapes_match_source() {
        compare::<u32>("/{id}/{other}", &["/1/2"]).await;
        compare::<(u32, u32)>("/{id}", &["/1"]).await;
        compare::<Params>("/{id}", &["/7"]).await;
        compare::<Option<u32>>("/{id}", &["/7"]).await;
        compare::<Nested>("/{id}", &["/7"]).await;
        compare::<Unsupported>("/{id}", &["/Value", "/Tuple", "/Fields"]).await;
        compare::<String>("/fixed", &["/fixed"]).await;
        compare::<()>("/fixed", &["/fixed"]).await;
        compare::<Vec<String>>("/fixed", &["/fixed"]).await;
        compare::<BTreeMap<u32, String>>("/{id}", &["/7"]).await;
    }
}
