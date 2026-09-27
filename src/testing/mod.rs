//! Optional HTTP test fixtures. No compatibility router conversion is required.
//!
//! Requests do not follow redirects or inherit proxies/cookies. Set Cookie and
//! Authorization headers explicitly. Buffered sends have a total timeout and a
//! response byte limit; infinite SSE streams should use a streaming client.
//!
//! ```
//! use simple_server::{testing::TestServer, web::{Router, routing::get}};
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), simple_server::testing::TestError> {
//! let server = TestServer::new(Router::new().route("/", get(|| async { "ok" })));
//! server.get("/").send().await?.assert_text("ok");
//! server.shutdown().await?;
//! # Ok(()) }
//! ```

use crate::{
    lifecycle::Shutdown,
    web::{Body, Router},
};
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};
use tower::ServiceExt;

pub use multipart::{MultipartForm, Part};
mod multipart;
#[cfg(feature = "test-harness-ws")]
mod websocket;
#[cfg(feature = "test-harness-ws")]
pub use websocket::{TestWebSocket, WebSocketOutcome};

/// Fallible fixture operations preserve their underlying error as a source.
pub type TestError = Box<dyn std::error::Error + Send + Sync>;
fn error(message: &str) -> TestError {
    std::io::Error::other(message).into()
}

/// Limits apply to each request, including response collection.
#[derive(Clone, Copy, Debug)]
pub struct TestOptions {
    pub timeout: Duration,
    pub max_response_bytes: usize,
}
impl Default for TestOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            max_response_bytes: 8 * 1024 * 1024,
        }
    }
}

enum Transport {
    InProcess(Router),
    Tcp(SocketAddr),
}

/// Owns a router fixture. TCP fixtures abort on drop; `shutdown` awaits graceful exit.
/// Construct and use TCP fixtures within the same Tokio runtime.
pub struct TestServer {
    transport: Transport,
    client: reqwest::Client,
    options: TestOptions,
    shutdown: Shutdown,
    task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}
impl TestServer {
    /// Invoke an owned router without opening a socket. Peer info/upgrades require TCP.
    pub fn new(router: Router) -> Self {
        Self {
            transport: Transport::InProcess(router),
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("test HTTP client"),
            options: TestOptions::default(),
            shutdown: Shutdown::new(),
            task: None,
        }
    }
    /// Bind a loopback port selected by the OS, including peer connection info.
    pub async fn tcp(router: Router) -> Result<Self, TestError> {
        Self::bind(router, SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await
    }
    /// Bind an explicit loopback address/port (useful for external test clients).
    pub async fn bind(router: Router, address: SocketAddr) -> Result<Self, TestError> {
        if !address.ip().is_loopback() {
            return Err(error("test server requires a loopback address"));
        }
        let listener = tokio::net::TcpListener::bind(address).await?;
        let mut server = Self::new(router.clone());
        server.transport = Transport::Tcp(listener.local_addr()?);
        server.task = Some(tokio::spawn(crate::web::serve_with_connect_info(
            listener,
            router,
            server.shutdown.clone(),
        )));
        Ok(server)
    }
    pub fn with_options(mut self, options: TestOptions) -> Self {
        self.options = options;
        self
    }
    pub fn address(&self) -> Option<SocketAddr> {
        match self.transport {
            Transport::Tcp(a) => Some(a),
            _ => None,
        }
    }
    pub fn base_url(&self) -> Option<String> {
        self.address().map(|a| format!("http://{a}"))
    }
    /// Graceful shutdown is bounded; expiry aborts the serving task.
    pub async fn shutdown(mut self) -> Result<(), TestError> {
        self.shutdown.request();
        if let Some(mut task) = self.task.take() {
            match tokio::time::timeout(self.options.timeout, &mut task).await {
                Ok(result) => result??,
                Err(e) => {
                    task.abort();
                    let _ = task.await;
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }
    pub fn request(&self, method: Method, path: &str) -> TestRequest<'_> {
        // Accept only origin-relative targets; a test request cannot escape its fixture.
        let invalid = !path.starts_with('/') || path.starts_with("//") || path.contains('#');
        let base = self.base_url().unwrap_or_else(|| "http://localhost".into());
        TestRequest {
            server: self,
            builder: self.client.request(method, format!("{base}{path}")),
            invalid,
            multipart: None,
        }
    }
    pub fn get(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::GET, path)
    }
    pub fn post(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::POST, path)
    }
    pub fn put(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PUT, path)
    }
    pub fn patch(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::PATCH, path)
    }
    pub fn delete(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::DELETE, path)
    }
    pub fn head(&self, path: &str) -> TestRequest<'_> {
        self.request(Method::HEAD, path)
    }
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.request();
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

/// Configure a request, then explicitly `send().await` to collect a bounded body.
pub struct TestRequest<'a> {
    server: &'a TestServer,
    builder: reqwest::RequestBuilder,
    invalid: bool,
    multipart: Option<MultipartForm>,
}
impl TestRequest<'_> {
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.builder = self.builder.header(name, value);
        self
    }
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.builder = self.builder.headers(headers);
        self
    }
    pub fn query<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
        self.builder = self.builder.query(value);
        self
    }
    pub fn json<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
        self.multipart = None;
        self.builder = self.builder.json(value);
        self
    }
    pub fn form<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
        self.multipart = None;
        self.builder = self.builder.form(value);
        self
    }
    pub fn body(mut self, value: impl Into<Vec<u8>>) -> Self {
        self.multipart = None;
        self.builder = self.builder.body(value.into());
        self
    }
    pub fn multipart(mut self, value: MultipartForm) -> Self {
        self.multipart = Some(value);
        self
    }
    fn build(self) -> Result<reqwest::Request, TestError> {
        if self.invalid {
            return Err(error(
                "request path must be origin-relative without a fragment",
            ));
        }
        let mut request = self.builder.build()?;
        if let Some(form) = self.multipart {
            let (content_type, bytes) = form.encode()?;
            request
                .headers_mut()
                .insert(http::header::CONTENT_TYPE, content_type.parse()?);
            *request.body_mut() = Some(bytes.into());
        }
        Ok(request)
    }
    pub async fn send(self) -> Result<TestResponse, TestError> {
        let server = self.server;
        let request = self.build()?;
        tokio::time::timeout(server.options.timeout, async {
            match &server.transport {
                Transport::InProcess(router) => {
                    let mut owned = http::Request::builder()
                        .method(request.method().clone())
                        .uri(format!(
                            "{}{}",
                            request.url().path(),
                            request
                                .url()
                                .query()
                                .map(|q| format!("?{q}"))
                                .unwrap_or_default()
                        ))
                        .body(Body::from(
                            request
                                .body()
                                .and_then(|b| b.as_bytes())
                                .unwrap_or_default()
                                .to_vec(),
                        ))?;
                    *owned.headers_mut() = request.headers().clone();
                    let response = router.clone().oneshot(owned).await?;
                    let (parts, body) = response.into_parts();
                    Ok(TestResponse {
                        status: parts.status,
                        headers: parts.headers,
                        body: body.collect(server.options.max_response_bytes).await?,
                    })
                }
                Transport::Tcp(_) => {
                    let mut response = server.client.execute(request).await?;
                    let status = response.status();
                    let headers = response.headers().clone();
                    let mut body = Vec::new();
                    while let Some(chunk) = response.chunk().await? {
                        if chunk.len()
                            > server.options.max_response_bytes.saturating_sub(body.len())
                        {
                            return Err(error("response exceeds test byte limit"));
                        }
                        body.extend_from_slice(&chunk);
                    }
                    Ok(TestResponse {
                        status,
                        headers,
                        body: body.into(),
                    })
                }
            }
        })
        .await?
    }
}

/// Buffered status, repeated headers and exact bytes, independent of the transport.
#[derive(Debug)]
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}
impl TestResponse {
    pub fn status_code(&self) -> StatusCode {
        self.status
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn as_bytes(&self) -> &Bytes {
        &self.body
    }
    pub fn text(&self) -> Result<&str, TestError> {
        Ok(std::str::from_utf8(&self.body)?)
    }
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, TestError> {
        Ok(serde_json::from_slice(&self.body)?)
    }
    #[track_caller]
    pub fn assert_status(&self, expected: StatusCode) {
        assert_eq!(self.status, expected, "response body: {:?}", self.body);
    }
    #[track_caller]
    pub fn assert_status_ok(&self) {
        self.assert_status(StatusCode::OK);
    }
    #[track_caller]
    pub fn assert_text(&self, expected: &str) {
        assert_eq!(self.text().expect("UTF-8 response"), expected);
    }
    #[track_caller]
    pub fn assert_json<T: Serialize>(&self, expected: &T) {
        assert_eq!(
            self.json::<serde_json::Value>().expect("JSON response"),
            serde_json::to_value(expected).expect("expected JSON")
        );
    }
}
