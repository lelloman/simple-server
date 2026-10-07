//! Blocking callers reuse the engine's asynchronous HTTP implementation.
//! Call only from ordinary or blocking-pool threads, never an async worker.
pub use super::multipart;
use super::{Error, Method, StatusCode};
use crate::runtime::Runtime;
use std::time::Duration;
#[derive(Clone)]
pub struct Client {
    inner: super::Client,
    runtime: Runtime,
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
impl Client {
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("blocking engine HTTP client")
    }
    pub fn builder() -> ClientBuilder {
        ClientBuilder(super::Client::builder())
    }
    pub fn request(&self, method: Method, url: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder {
            inner: self.inner.request(method, url),
            runtime: self.runtime.clone(),
        }
    }
    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::GET, url)
    }
    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::POST, url)
    }
}
pub struct ClientBuilder(super::ClientBuilder);
impl ClientBuilder {
    pub fn timeout(self, value: Duration) -> Self {
        Self(self.0.timeout(value))
    }
    pub fn user_agent(self, value: impl AsRef<str>) -> Self {
        Self(self.0.user_agent(value))
    }
    pub fn no_proxy(self) -> Self {
        Self(self.0.no_proxy())
    }
    pub fn no_redirect(self) -> Self {
        Self(self.0.no_redirect())
    }
    pub fn build(self) -> Result<Client, Error> {
        let runtime = Runtime::try_current()
            .or_else(|_| Runtime::new())
            .map_err(Error::local)?;
        let inner = runtime.with_current(|| self.0.build())?;
        Ok(Client { inner, runtime })
    }
}
pub struct RequestBuilder {
    inner: super::RequestBuilder,
    runtime: Runtime,
}
impl RequestBuilder {
    pub fn bearer_auth(mut self, value: impl std::fmt::Display) -> Self {
        self.inner = self.inner.bearer_auth(value);
        self
    }
    pub fn multipart(mut self, form: multipart::Form) -> Self {
        self.inner = self.inner.multipart(form);
        self
    }
    pub fn json<T: serde::Serialize + ?Sized>(mut self, value: &T) -> Self {
        self.inner = self.inner.json(value);
        self
    }
    pub fn send(self) -> Result<Response, Error> {
        let inner = futures_executor::block_on(self.runtime.scope(self.inner.send()))?;
        Ok(Response {
            inner,
            runtime: self.runtime,
        })
    }
}
pub struct Response {
    inner: super::Response,
    runtime: Runtime,
}
impl Response {
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }
    pub fn text(self) -> Result<String, Error> {
        futures_executor::block_on(self.runtime.scope(self.inner.text()))
    }
    pub fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, Error> {
        futures_executor::block_on(self.runtime.scope(self.inner.json()))
    }
    pub fn error_for_status(self) -> Result<Self, Error> {
        Ok(Self {
            inner: self.inner.error_for_status()?,
            runtime: self.runtime,
        })
    }
}
