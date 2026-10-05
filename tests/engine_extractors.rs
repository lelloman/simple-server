#![cfg(feature = "engine-web")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use simple_server::engine_web::IntoResponse;
use simple_server::engine_web::{self as e, FromRequest, FromRequestParts, Handler};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Input {
    count: u32,
}
fn request(body: impl Into<e::Body>, content: Option<&str>) -> e::Request {
    let mut request = e::Request::new(body.into());
    if let Some(value) = content {
        request
            .headers_mut()
            .insert("content-type", value.parse().unwrap());
    }
    request
}
async fn parts(response: e::Response) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect(4 * 1024 * 1024).await.unwrap().to_vec(),
    )
}
#[simple_server::test]
async fn json_presence_errors_and_default_body_limit_are_explicit() {
    assert!(
        Option::<e::Json<Input>>::from_request(request("broken", None), &())
            .await
            .unwrap()
            .is_none()
    );
    for (value, kind, status) in [
        ("{}", None, e::StatusCode::UNSUPPORTED_MEDIA_TYPE),
        (
            "{}",
            Some("text/json"),
            e::StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        ("{", Some("application/json"), e::StatusCode::BAD_REQUEST),
        (
            "{\"count\":\"bad\"}",
            Some("application/json"),
            e::StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "{\"count\":1} 2",
            Some("application/json"),
            e::StatusCode::BAD_REQUEST,
        ),
    ] {
        let rejection = e::Json::<Input>::from_request(request(value, kind), &())
            .await
            .unwrap_err();
        assert_eq!(rejection.status(), status);
        assert_eq!(
            rejection.headers()["content-type"],
            "text/plain; charset=utf-8"
        );
    }
    let result = e::Json::<Input>::from_request(
        request(
            "{\"count\":7}",
            Some("application/problem+json; charset=utf-8"),
        ),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(result.0.count, 7);
    assert!(
        e::Bytes::from_request(request(vec![0; 2 * 1024 * 1024], None), &())
            .await
            .is_ok()
    );
    assert_eq!(
        e::Bytes::from_request(request(vec![0; 2 * 1024 * 1024 + 1], None), &())
            .await
            .unwrap_err()
            .status(),
        e::StatusCode::PAYLOAD_TOO_LARGE
    );
    let mut limited = request("abc", None);
    limited.extensions_mut().insert(e::BodyLimit(2));
    assert_eq!(
        String::from_request(limited, &())
            .await
            .unwrap_err()
            .status(),
        e::StatusCode::PAYLOAD_TOO_LARGE
    );
}
struct Deny;
impl<S: Sync> FromRequestParts<S> for Deny {
    type Rejection = simple_server::extract::RejectionResponse;
    async fn from_request_parts(
        _: &mut simple_server::extract::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        use simple_server::extract::IntoRejectionResponse;
        let mut rejection = (e::StatusCode::UNAUTHORIZED, "denied").into_rejection_response();
        rejection
            .headers_mut()
            .insert("x-reason", "custom".parse().unwrap());
        Err(rejection)
    }
}
#[simple_server::test]
async fn custom_head_rejection_short_circuits_body_and_preserves_headers() {
    let polls = Arc::new(AtomicUsize::new(0));
    let count = polls.clone();
    let stream = futures_util::stream::poll_fn(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
        std::task::Poll::Ready(Some(Ok::<_, std::io::Error>(e::Bytes::from_static(b"{}"))))
    });
    async fn handler(_: e::Extract<Deny>, _: e::Json<Input>) -> &'static str {
        panic!("rejected handler ran")
    }
    let response = handler
        .call(
            request(e::Body::from_stream(stream), Some("application/json")),
            (),
        )
        .await;
    let (status, headers, body) = parts(response).await;
    assert_eq!(status, e::StatusCode::UNAUTHORIZED);
    assert_eq!(headers["x-reason"], "custom");
    assert_eq!(body, b"denied");
    assert_eq!(polls.load(Ordering::SeqCst), 0);
}
#[simple_server::test]
async fn result_capture_queries_state_and_zero_argument_handlers_work() {
    async fn handler(
        e::State(prefix): e::State<String>,
        query: Result<e::Query<Input>, e::RejectionResponse>,
        e::RawQuery(raw): e::RawQuery,
    ) -> String {
        assert_eq!(raw.as_deref(), Some("count=bad"));
        format!("{prefix}:{}", query.unwrap_err().status().as_u16())
    }
    let mut req = request("", None);
    *req.uri_mut() = "/?count=bad".parse().unwrap();
    assert_eq!(
        parts(handler.call(req, "state".to_owned()).await).await.2,
        b"state:400"
    );
    let response = (|| async { (e::StatusCode::ACCEPTED, "zero") })
        .call(request("", None), ())
        .await;
    assert_eq!(parts(response).await.0, e::StatusCode::ACCEPTED);
    async fn captured(body: Result<e::Json<Input>, e::RejectionResponse>) -> e::StatusCode {
        body.unwrap_err().status()
    }
    assert_eq!(
        parts(
            captured
                .call(request("no", Some("application/json")), ())
                .await
        )
        .await
        .0,
        e::StatusCode::BAD_REQUEST
    );
}
#[cfg(feature = "client")]
#[simple_server::test]
async fn typed_registered_handler_runs_through_engine_with_state_and_metadata() {
    use simple_server::{client::Client, runtime::spawn, time::timeout};
    async fn handler(
        e::State(prefix): e::State<String>,
        matched: e::MatchedPath,
        e::ConnectInfo(peer): e::ConnectInfo<std::net::SocketAddr>,
        e::Query(query): e::Query<Input>,
        e::Json(body): e::Json<Input>,
    ) -> (e::StatusCode, e::Json<serde_json::Value>) {
        assert_eq!(matched.as_str(), "/typed");
        assert!(peer.ip().is_loopback());
        (
            e::StatusCode::CREATED,
            e::Json(serde_json::json!({"prefix":prefix,"count":query.count+body.count})),
        )
    }
    let router = e::Router::new()
        .unwrap()
        .route(
            "/typed",
            e::MethodRouter::new()
                .unwrap()
                .on_state(e::Method::POST, "engine".to_owned(), handler)
                .unwrap(),
        )
        .unwrap()
        .fallback_handler(|| async { (e::StatusCode::NOT_FOUND, "missing") })
        .unwrap();
    let listener = e::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/typed?count=3", listener.local_addr());
    let stop = e::Shutdown::new();
    let server = spawn(e::serve(listener, router, stop.clone()));
    let response = Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(url)
        .json(&Input { count: 4 })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), e::StatusCode::CREATED);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap(),
        serde_json::json!({"prefix":"engine","count":7})
    );
    stop.request();
    timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
#[cfg(feature = "web")]
mod parity {
    use super::*;
    use simple_server::web as s;
    async fn source_parts(response: s::Response) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
        let (parts, body) = response.into_parts();
        (
            parts.status,
            parts.headers,
            body.collect(4 * 1024 * 1024).await.unwrap().to_vec(),
        )
    }
    fn source_request(body: Vec<u8>, content: Option<&str>) -> s::Request {
        let mut request = s::Request::new(s::Body::from(body));
        if let Some(value) = content {
            request
                .headers_mut()
                .insert("content-type", value.parse().unwrap());
        }
        request
    }
    #[simple_server::test]
    async fn json_rejection_status_headers_and_text_match_source() {
        for (body, content) in [
            ("{}", None),
            ("{}", Some("text/plain")),
            ("{", Some("application/json")),
            ("{\"count\":\"bad\"}", Some("application/json")),
            ("{\"count\":1} true", Some("application/json")),
            ("{\"count\":1}", Some("application/json")),
        ] {
            let engine = e::Json::<Input>::from_request(request(body, content), &()).await;
            let source = <s::Json<Input> as s::FromRequest<()>>::from_request(
                source_request(body.as_bytes().to_vec(), content),
                &(),
            )
            .await;
            let engine = match engine {
                Ok(value) => value.into_response(),
                Err(error) => error.into_response(),
            };
            let source = match source {
                Ok(value) => s::IntoResponse::into_response(value),
                Err(error) => s::IntoResponse::into_response(error),
            };
            assert_eq!(
                parts(engine).await,
                source_parts(source).await,
                "{body:?} {content:?}"
            );
        }
    }
    #[simple_server::test]
    async fn query_and_utf8_rejections_match_source() {
        for query in [
            "",
            "?count=bad",
            "?count=2",
            "?count=1&count=2",
            "?count=%FF",
        ] {
            let uri: e::Uri = format!("/{query}").parse().unwrap();
            let (mut ep, _) = request("", None).into_parts();
            ep.uri = uri.clone();
            let (mut sp, _) = source_request(Vec::new(), None).into_parts();
            sp.uri = uri;
            let engine = e::Query::<Input>::from_request_parts(&mut ep, &())
                .await
                .map(|v| e::Json(v.0))
                .map_err(IntoResponse::into_response);
            let source = s::Query::<Input>::from_request_parts(&mut sp, &())
                .await
                .map(|v| s::Json(v.0))
                .map_err(s::IntoResponse::into_response);
            assert_eq!(
                parts(engine.into_response()).await,
                source_parts(s::IntoResponse::into_response(source)).await,
                "{query}"
            );
        }
        let bytes = vec![0xff];
        let engine = String::from_request(request(bytes.clone(), None), &()).await;
        let source =
            <String as s::FromRequest<()>>::from_request(source_request(bytes, None), &()).await;
        assert_eq!(
            parts(engine.into_response()).await,
            source_parts(s::IntoResponse::into_response(source)).await
        );
    }
    #[simple_server::test]
    async fn body_limit_and_stream_failure_match_source() {
        let mut er = request("large", None);
        er.extensions_mut().insert(e::BodyLimit(2));
        let sr = source_request(b"large".to_vec(), None);
        // Apply the source framework's extractor limit through its public layer.
        use tower_layer::Layer;
        use tower_service::Service;
        let inner = tower::service_fn(|r: s::Request| async {
            Ok::<_, std::convert::Infallible>(s::IntoResponse::into_response(
                <String as s::FromRequest<()>>::from_request(r, &()).await,
            ))
        });
        // The layer targets the owned request type, so the extension remains available to the adapter.
        let mut service = axum::extract::DefaultBodyLimit::max(2).layer(inner);
        let source = service.call(sr).await.unwrap();
        assert_eq!(
            parts(String::from_request(er, &()).await.into_response()).await,
            source_parts(source).await
        );
        let engine = e::Body::from_stream(futures_util::stream::iter([Err::<e::Bytes, _>(
            std::io::Error::other("broken"),
        )]));
        let source = s::Body::from_stream(futures_util::stream::iter([Err::<e::Bytes, _>(
            std::io::Error::other("broken"),
        )]));
        let engine = String::from_request(request(engine, None), &()).await;
        let source =
            <String as s::FromRequest<()>>::from_request(s::Request::new(source), &()).await;
        assert_eq!(
            parts(engine.into_response()).await,
            source_parts(s::IntoResponse::into_response(source)).await
        );
    }
}

#[cfg(feature = "web")]
#[simple_server::test]
async fn response_conversions_match_source_including_serialization_failure() {
    use simple_server::web as s;
    async fn source(response: s::Response) -> (e::StatusCode, e::HeaderMap, Vec<u8>) {
        let (parts, body) = response.into_parts();
        (
            parts.status,
            parts.headers,
            body.collect(1024).await.unwrap().to_vec(),
        )
    }
    macro_rules! compare {
        ($engine:expr,$source:expr) => {
            assert_eq!(
                parts(e::IntoResponse::into_response($engine)).await,
                source(s::IntoResponse::into_response($source)).await
            );
        };
    }
    compare!((), ());
    compare!("text", "text");
    compare!(vec![0, 255], vec![0u8, 255]);
    compare!(
        (e::StatusCode::CREATED, "created"),
        (e::StatusCode::CREATED, "created")
    );
    compare!(
        e::response::Html("<b>html</b>"),
        s::response::Html("<b>html</b>")
    );
    let mut headers = e::HeaderMap::new();
    headers.append("set-cookie", "a=1".parse().unwrap());
    headers.append("set-cookie", "b=2".parse().unwrap());
    compare!(
        (e::StatusCode::ACCEPTED, headers.clone(), "ok"),
        (e::StatusCode::ACCEPTED, headers, "ok")
    );
    let bad = std::collections::BTreeMap::from([(vec![1u8], 2u8)]);
    compare!(e::Json(bad.clone()), s::Json(bad));
}
