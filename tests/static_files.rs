#![cfg(feature = "static-files")]

use simple_server::web::{
    Body, HeaderMap, Request, Response, Router, StatusCode,
    routing::get,
    static_files::{StaticDir, StaticFile},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "simple-server-static-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::create_dir(path.join("site")).unwrap();
        std::fs::create_dir(path.join("site/docs")).unwrap();
        for (name, body) in [
            ("site/index.html", "<h1>SPA</h1>"),
            ("site/docs/index.html", "documentation"),
            ("site/app.js", "0123456789"),
            ("site/hello world.txt", "space"),
            ("secret.txt", "outside secret"),
        ] {
            std::fs::write(path.join(name), body).unwrap();
        }
        Self(path)
    }
    fn root(&self) -> PathBuf {
        self.0.join("site")
    }
    fn spa(&self) -> StaticDir {
        StaticDir::new(self.root()).fallback_file(self.root().join("index.html"))
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn request(method: &str, uri: &str, headers: &[(&str, &str)]) -> Request {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.body(Body::empty()).unwrap()
}
async fn snapshot(response: Response) -> (StatusCode, HeaderMap, Vec<u8>) {
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect(2 * 1024 * 1024).await.unwrap().to_vec(),
    )
}

#[tokio::test]
async fn assets_head_mime_and_query_strings() {
    let files = Files::new();
    let app = Router::new().fallback_service(StaticDir::new(files.root()));
    let (status, headers, body) = snapshot(
        app.clone()
            .oneshot(request("GET", "/app.js?v=2", &[]))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"0123456789");
    assert!(
        headers["content-type"]
            .to_str()
            .unwrap()
            .contains("javascript")
    );
    assert_eq!(headers["content-length"], "10");
    let (status, head_headers, body) =
        snapshot(app.oneshot(request("HEAD", "/app.js", &[])).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(head_headers, headers);
    assert!(body.is_empty());
}

#[tokio::test]
async fn directory_index_redirect_and_disable() {
    let files = Files::new();
    let app = Router::new().fallback_service(StaticDir::new(files.root()));
    let (status, _, body) =
        snapshot(app.clone().oneshot(request("GET", "/", &[])).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>SPA</h1>");
    let (status, headers, _) = snapshot(
        app.clone()
            .oneshot(request("GET", "/docs?x=1", &[]))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(headers["location"], "/docs/?x=1");
    let (_, _, body) = snapshot(app.oneshot(request("GET", "/docs/", &[])).await.unwrap()).await;
    assert_eq!(body, b"documentation");
    let response = StaticDir::new(files.root())
        .append_index_html_on_directories(false)
        .oneshot(request("GET", "/", &[]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn explicit_spa_missing_assets_and_missing_fallback() {
    let files = Files::new();
    let app = Router::new()
        .route("/api", get(|| async { "api" }))
        .fallback_service(files.spa());
    for path in ["/albums/123", "/missing.js"] {
        let (status, _, body) = snapshot(
            app.clone()
                .oneshot(request("GET", path, &[]))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"<h1>SPA</h1>");
    }
    let (_, _, body) = snapshot(app.oneshot(request("GET", "/api", &[])).await.unwrap()).await;
    assert_eq!(body, b"api");
    assert_eq!(
        StaticDir::new(files.root())
            .oneshot(request("GET", "/missing", &[]))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        StaticDir::new(files.root())
            .fallback_file(files.root().join("absent.html"))
            .oneshot(request("GET", "/missing", &[]))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn error_page_is_404_and_fallback_can_be_replaced() {
    let files = Files::new();
    let (status, _, body) = snapshot(
        StaticDir::new(files.root())
            .fallback_file(files.root().join("absent"))
            .not_found_file(files.root().join("index.html"))
            .oneshot(request("GET", "/missing", &[]))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, b"<h1>SPA</h1>");
    let response = StaticDir::new(files.root())
        .not_found_file(files.root().join("index.html"))
        .oneshot(request("GET", "/app.js", &[]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ranges_and_modification_conditionals() {
    let files = Files::new();
    let service = StaticFile::new(files.root().join("app.js"));
    for (range, expected) in [
        ("bytes=2-5", "2345"),
        ("bytes=-3", "789"),
        ("bytes=7-", "789"),
    ] {
        let (status, headers, body) = snapshot(
            service
                .clone()
                .oneshot(request("GET", "/any", &[("range", range)]))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(body, expected.as_bytes());
        assert!(headers.contains_key("content-range"));
    }
    let response = service
        .clone()
        .oneshot(request("GET", "/", &[("range", "bytes=99-")]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(response.headers()["content-range"], "bytes */10");
    let first = service
        .clone()
        .oneshot(request("GET", "/", &[]))
        .await
        .unwrap();
    let modified = first.headers()["last-modified"].to_str().unwrap();
    let response = service
        .clone()
        .oneshot(request("GET", "/", &[("if-modified-since", modified)]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert!(response.into_body().collect(1024).await.unwrap().is_empty());
    let response = service
        .oneshot(request(
            "GET",
            "/",
            &[("if-unmodified-since", "Wed, 21 Oct 2015 07:28:00 GMT")],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
}

#[tokio::test]
async fn unsupported_methods_do_not_serve_spa_or_single_file() {
    let files = Files::new();
    for method in ["POST", "PUT", "DELETE", "OPTIONS"] {
        let response = files
            .spa()
            .oneshot(request(method, "/missing", &[]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers()["allow"], "GET,HEAD");
        assert_eq!(
            StaticFile::new(files.root().join("index.html"))
                .oneshot(request(method, "/", &[]))
                .await
                .unwrap()
                .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
}

#[tokio::test]
async fn traversal_never_reads_outside_root() {
    let files = Files::new();
    for path in [
        "/../secret.txt",
        "/%2e%2e/secret.txt",
        "/%2E%2E%2Fsecret.txt",
        "/..%5csecret.txt",
        "/%5csecret.txt",
        "/app.js/nested",
    ] {
        let response = StaticDir::new(files.root())
            .oneshot(request("GET", path, &[]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        let (_, _, body) = snapshot(
            files
                .spa()
                .oneshot(request("GET", path, &[]))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(body, b"<h1>SPA</h1>", "{path}");
    }
    let (_, _, body) = snapshot(
        StaticDir::new(files.root())
            .oneshot(request("GET", "/hello%20world.txt", &[]))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(body, b"space");
}

#[tokio::test]
async fn nested_mount_strips_prefix_and_keeps_api_independent() {
    let files = Files::new();
    let app = Router::new()
        .nest_service("/assets", StaticDir::new(files.root()))
        .route("/api", get(|| async { "api" }));
    let (status, _, body) = snapshot(
        app.clone()
            .oneshot(request("GET", "/assets/app.js", &[]))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"0123456789");
    assert_eq!(
        app.clone()
            .oneshot(request("GET", "/app.js", &[]))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let (_, _, body) = snapshot(app.oneshot(request("GET", "/api", &[])).await.unwrap()).await;
    assert_eq!(body, b"api");
}

#[tokio::test]
async fn precompressed_variants_are_explicit_and_negotiated() {
    let files = Files::new();
    std::fs::write(files.root().join("app.js.gz"), b"gzip fixture").unwrap();
    std::fs::write(files.root().join("app.js.br"), b"brotli fixture").unwrap();
    let service = StaticDir::new(files.root())
        .precompressed_gzip()
        .precompressed_br();
    for (encoding, expected) in [("gzip", "gzip fixture"), ("br", "brotli fixture")] {
        let (_, headers, body) = snapshot(
            service
                .clone()
                .oneshot(request("GET", "/app.js", &[("accept-encoding", encoding)]))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(headers["content-encoding"], encoding);
        assert_eq!(body, expected.as_bytes());
    }
    let (_, headers, body) = snapshot(
        service
            .oneshot(request("GET", "/app.js", &[]))
            .await
            .unwrap(),
    )
    .await;
    assert!(!headers.contains_key("content-encoding"));
    assert_eq!(body, b"0123456789");
    let (_, headers, body) = snapshot(
        StaticFile::new(files.root().join("app.js"))
            .precompressed_gzip()
            .precompressed_br()
            .oneshot(request("GET", "/", &[("accept-encoding", "gzip")]))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(headers["content-encoding"], "gzip");
    assert_eq!(body, b"gzip fixture");
}

#[tokio::test]
async fn spa_contract_matches_previous_backend() {
    let files = Files::new();
    let old = axum::Router::new().fallback_service(
        tower_http::services::ServeDir::new(files.root())
            .append_index_html_on_directories(true)
            .fallback(tower_http::services::ServeFile::new(
                files.root().join("index.html"),
            )),
    );
    let new = Router::new().fallback_service(files.spa());
    for method in ["GET", "HEAD", "POST"] {
        for uri in [
            "/",
            "/app.js?x=1",
            "/docs",
            "/docs/",
            "/albums/123",
            "/missing.js",
            "/%2e%2e/secret.txt",
        ] {
            for headers in [&[][..], &[("range", "bytes=0-2")][..]] {
                let old_response = old
                    .clone()
                    .oneshot(request(method, uri, headers).map(|_| axum::body::Body::empty()))
                    .await
                    .unwrap();
                let before = snapshot(old_response.map(Body::new)).await;
                let after = snapshot(
                    new.clone()
                        .oneshot(request(method, uri, headers))
                        .await
                        .unwrap(),
                )
                .await;
                assert_eq!(after, before, "{method} {uri} {headers:?}");
            }
        }
    }
}

#[cfg(feature = "test-harness")]
#[tokio::test]
async fn real_tcp_serving_and_shutdown() {
    use simple_server::testing::TestServer;
    let files = Files::new();
    let server = TestServer::tcp(
        Router::new()
            .route("/api", get(|| async { "api" }))
            .fallback_service(files.spa()),
    )
    .await
    .unwrap();
    server
        .get("/albums/123")
        .send()
        .await
        .unwrap()
        .assert_text("<h1>SPA</h1>");
    let head = server.head("/app.js").send().await.unwrap();
    head.assert_status_ok();
    assert!(head.as_bytes().is_empty());
    assert_eq!(head.headers()["content-length"], "10");
    let range = server
        .get("/app.js")
        .header("range", "bytes=3-5")
        .send()
        .await
        .unwrap();
    range.assert_status(StatusCode::PARTIAL_CONTENT);
    range.assert_text("345");
    server.get("/api").send().await.unwrap().assert_text("api");
    server.shutdown().await.unwrap();
}
