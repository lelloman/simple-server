#![cfg(feature = "test-harness")]
use simple_server::{
    testing::{MultipartForm, Part, TestOptions, TestServer},
    web::{self, Body, IntoResponse, Request, Router, StatusCode},
};
use std::time::Duration;

fn app() -> Router {
    Router::new()
        .route(
            "/echo",
            web::routing::post(|request: Request| async {
                let headers = request.headers().clone();
                let bytes = request.into_body().collect(4096).await.unwrap();
                (headers, bytes).into_response()
            }),
        )
        .route(
            "/query",
            web::routing::get(
                |web::Query(q): web::Query<std::collections::BTreeMap<String, String>>| async move {
                    web::Json(q)
                },
            ),
        )
        .route(
            "/redirect",
            web::routing::get(|| async {
                (StatusCode::FOUND, [("location", "/query")], "redirect")
            }),
        )
        .route(
            "/cookies",
            web::routing::get(|| async {
                {
                    let mut headers = web::HeaderMap::new();
                    headers.append("set-cookie", "a=1".parse().unwrap());
                    headers.append("set-cookie", "b=2".parse().unwrap());
                    (headers, "cookies")
                }
            }),
        )
        .route("/bytes", web::routing::get(|| async { vec![0, 255, 1] }))
        .route(
            "/slow",
            web::routing::get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                "late"
            }),
        )
}
async fn contract(server: TestServer) {
    let response = server
        .post("/echo")
        .header("authorization", "Bearer test")
        .json(&serde_json::json!({"unicode":"héllo"}))
        .send()
        .await
        .unwrap();
    response.assert_status_ok();
    response.assert_json(&serde_json::json!({"unicode":"héllo"}));
    assert_eq!(response.headers()["authorization"], "Bearer test");
    assert_eq!(response.headers()["content-type"], "application/json");
    server
        .get("/query")
        .query(&[("x", "a +&☃")])
        .send()
        .await
        .unwrap()
        .assert_json(&serde_json::json!({"x":"a +&☃"}));
    let response = server
        .post("/echo")
        .form(&[("x", "a +&")])
        .send()
        .await
        .unwrap();
    response.assert_text("x=a+%2B%26");
    assert_eq!(
        response.headers()["content-type"],
        "application/x-www-form-urlencoded"
    );
    server
        .get("/redirect")
        .send()
        .await
        .unwrap()
        .assert_status(StatusCode::FOUND);
    let response = server.get("/cookies").send().await.unwrap();
    assert_eq!(response.headers().get_all("set-cookie").iter().count(), 2);
    let response = server.post("/echo").send().await.unwrap();
    assert!(response.headers().get("cookie").is_none());
    let response = server.get("/bytes").send().await.unwrap();
    assert_eq!(response.as_bytes().as_ref(), &[0, 255, 1]);
    assert!(response.text().is_err());
    assert!(response.json::<serde_json::Value>().is_err());
    assert!(
        server
            .head("/bytes")
            .send()
            .await
            .unwrap()
            .as_bytes()
            .is_empty()
    );
    server
        .get("/missing")
        .send()
        .await
        .unwrap()
        .assert_status(StatusCode::NOT_FOUND);
    assert!(server.get("http://example.com").send().await.is_err());
    assert!(server.get("//example.com").send().await.is_err());
    assert!(server.get("/query#fragment").send().await.is_err());
    assert!(
        server
            .get("/")
            .header("bad\nname", "x")
            .send()
            .await
            .is_err()
    );
    server.shutdown().await.unwrap();
}
#[tokio::test]
async fn in_process_contract() {
    contract(TestServer::new(app())).await;
}
#[tokio::test]
async fn tcp_contract() {
    contract(TestServer::tcp(app()).await.unwrap()).await;
}
async fn limits(tcp: bool) {
    let server = if tcp {
        TestServer::tcp(app()).await.unwrap()
    } else {
        TestServer::new(app())
    };
    let server = server.with_options(TestOptions {
        timeout: Duration::from_millis(20),
        max_response_bytes: 2,
    });
    assert!(server.get("/bytes").send().await.is_err());
    assert!(server.get("/slow").send().await.is_err());
    server.shutdown().await.unwrap();
}
#[tokio::test]
async fn in_process_limits() {
    limits(false).await;
}
#[tokio::test]
async fn tcp_limits() {
    limits(true).await;
}
#[tokio::test]
async fn tcp_external_client_and_shutdown_release_listener() {
    let server = TestServer::tcp(app()).await.unwrap();
    let addr = server.address().unwrap();
    let response = reqwest::get(format!("{}/bytes", server.base_url().unwrap()))
        .await
        .unwrap();
    assert_eq!(response.bytes().await.unwrap().as_ref(), &[0, 255, 1]);
    server.shutdown().await.unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    drop(listener);
}
#[tokio::test]
async fn drop_releases_listener() {
    let server = TestServer::tcp(app()).await.unwrap();
    let addr = server.address().unwrap();
    drop(server);
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
                drop(listener);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn rejects_non_loopback() {
    assert!(
        TestServer::bind(app(), "0.0.0.0:0".parse().unwrap())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn body_stream_is_bounded() {
    let app = Router::new().route(
        "/",
        web::routing::get(|| async {
            Body::from_stream(futures_util::stream::pending::<
                Result<bytes::Bytes, std::io::Error>,
            >())
        }),
    );
    let server = TestServer::new(app).with_options(TestOptions {
        timeout: Duration::from_millis(20),
        ..Default::default()
    });
    assert!(server.get("/").send().await.is_err());
}
#[tokio::test]
async fn multipart_encoding_metadata_validation() {
    let server = TestServer::new(app());
    let response = server
        .post("/echo")
        .multipart(
            MultipartForm::new()
                .add_text("x", "one")
                .add_text("x", "two")
                .add_part(
                    "file",
                    Part::bytes(vec![0, 255])
                        .file_name("a\"b.bin")
                        .mime_type("application/octet-stream"),
                ),
        )
        .send()
        .await
        .unwrap();
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("multipart/form-data; boundary=")
    );
    assert!(response.as_bytes().windows(2).any(|b| b == [0, 255]));
    for name in ["bad\r\nname", "bad\0name"] {
        assert!(
            server
                .post("/echo")
                .multipart(MultipartForm::new().add_text(name, "x"))
                .send()
                .await
                .is_err()
        );
    }
    assert!(
        server
            .post("/echo")
            .multipart(
                MultipartForm::new().add_part("x", Part::bytes(vec![]).mime_type("x\r\ny: z"))
            )
            .send()
            .await
            .is_err()
    );
}
#[cfg(feature = "multipart")]
#[tokio::test]
async fn multipart_parses_in_both_transports() {
    async fn upload(mut m: web::multipart::Multipart) -> web::Json<serde_json::Value> {
        let mut fields = Vec::new();
        while let Some(field) = m.next_field().await.unwrap() {
            let name = field.name().unwrap().to_owned();
            let filename = field.file_name().map(str::to_owned);
            let mime = field.content_type().map(str::to_owned);
            fields.push(serde_json::json!([
                name,
                filename,
                mime,
                field.bytes().await.unwrap().to_vec()
            ]));
        }
        web::Json(serde_json::json!(fields))
    }
    let app = Router::new().route("/", web::routing::post(upload));
    for tcp in [false, true] {
        let server = if tcp {
            TestServer::tcp(app.clone()).await.unwrap()
        } else {
            TestServer::new(app.clone())
        };
        server
            .post("/")
            .multipart(
                MultipartForm::new()
                    .add_text("x", "one")
                    .add_text("x", "two")
                    .add_part(
                        "file",
                        Part::bytes(vec![0, 255])
                            .file_name("test.bin")
                            .mime_type("application/octet-stream"),
                    ),
            )
            .send()
            .await
            .unwrap()
            .assert_json(&serde_json::json!([
                ["x", null, null, [111, 110, 101]],
                ["x", null, null, [116, 119, 111]],
                ["file", "test.bin", "application/octet-stream", [0, 255]]
            ]));
        server.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn peer_connection_info_in_tcp_mode() {
    let router = Router::new().route(
        "/",
        web::routing::get(
            |web::ConnectInfo(peer): web::ConnectInfo<std::net::SocketAddr>| async move {
                peer.ip().to_string()
            },
        ),
    );
    let server = TestServer::tcp(router).await.unwrap();
    server
        .get("/")
        .send()
        .await
        .unwrap()
        .assert_text("127.0.0.1");
    server.shutdown().await.unwrap();
}

#[test]
fn in_process_construction_needs_no_runtime() {
    let server = TestServer::new(app());
    assert!(server.address().is_none());
    assert!(server.base_url().is_none());
}
#[tokio::test]
async fn graceful_shutdown_has_a_deadline() {
    let router = Router::new().route(
        "/",
        web::routing::get(|| async {
            Body::from_stream(
                futures_util::stream::once(async {
                    Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"start"))
                })
                .chain(futures_util::stream::pending()),
            )
        }),
    );
    use futures_util::StreamExt;
    let server = TestServer::tcp(router)
        .await
        .unwrap()
        .with_options(TestOptions {
            timeout: Duration::from_millis(20),
            ..Default::default()
        });
    let response = reqwest::get(format!("{}/", server.base_url().unwrap()))
        .await
        .unwrap();
    assert!(server.shutdown().await.is_err());
    drop(response);
}
