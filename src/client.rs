//! Outbound HTTP with a prebuilt engine implementation.
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, header::HeaderName};
pub use http::{Method, StatusCode, header};
pub mod blocking;
pub mod multipart;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use simple_server_sys::Resource;
use std::{fmt, time::Duration};

#[derive(Debug)]
pub struct Error {
    message: String,
    timeout: bool,
    connect: bool,
    status: Option<StatusCode>,
}
impl Error {
    fn local(error: impl fmt::Display) -> Self {
        Self {
            message: error.to_string(),
            timeout: false,
            connect: false,
            status: None,
        }
    }
    pub fn is_timeout(&self) -> bool {
        self.timeout
    }
    pub fn is_connect(&self) -> bool {
        self.connect
    }
    pub fn status(&self) -> Option<StatusCode> {
        self.status
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
fn checked(header: Value) -> Result<Value, Error> {
    if header["ok"] == true {
        Ok(header)
    } else {
        Err(Error {
            message: header["message"]
                .as_str()
                .unwrap_or("engine HTTP failure")
                .to_owned(),
            timeout: header["timeout"] == true,
            connect: header["connect"] == true,
            status: None,
        })
    }
}
fn millis(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

#[derive(Clone)]
pub struct Client {
    resource: Resource,
}
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
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
            .expect("cannot create engine HTTP client")
    }
    pub fn builder() -> ClientBuilder {
        ClientBuilder {
            config: json!({"op":"client"}),
        }
    }
    pub fn request(&self, method: Method, url: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder {
            client: self.clone(),
            request: Ok(RequestData {
                method,
                url: url.as_ref().to_owned(),
                headers: HeaderMap::new(),
                body: Vec::new(),
                multipart: None,
                timeout: None,
            }),
        }
    }
    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::GET, url)
    }
    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::POST, url)
    }
    pub fn put(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::PUT, url)
    }
    pub fn delete(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }
    pub fn head(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }
    pub fn patch(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }
}
pub struct ClientBuilder {
    config: Value,
}
impl ClientBuilder {
    pub fn timeout(mut self, value: Duration) -> Self {
        self.config["timeout_ms"] = millis(value).into();
        self
    }
    pub fn read_timeout(mut self, value: Duration) -> Self {
        self.config["read_timeout_ms"] = json!(millis(value));
        self
    }
    pub fn connect_timeout(mut self, value: Duration) -> Self {
        self.config["connect_timeout_ms"] = millis(value).into();
        self
    }
    pub fn user_agent(mut self, value: impl AsRef<str>) -> Self {
        self.config["user_agent"] = value.as_ref().into();
        self
    }
    pub fn https_only(mut self, value: bool) -> Self {
        self.config["https_only"] = json!(value);
        self
    }
    pub fn no_proxy(mut self) -> Self {
        self.config["no_proxy"] = true.into();
        self
    }
    /// Disable automatic response decompression, preserving upstream bytes.
    pub fn no_decompression(mut self) -> Self {
        self.config["no_decompression"] = true.into();
        self
    }
    pub fn no_redirect(mut self) -> Self {
        self.config["no_redirect"] = true.into();
        self
    }
    pub fn build(self) -> Result<Client, Error> {
        let command = serde_json::to_vec(&self.config).map_err(Error::local)?;
        let result = simple_server_sys::resource_new(&command).map_err(Error::local)?;
        let (header, _) = crate::engine_wire::decode(&result).map_err(Error::local)?;
        let header = checked(header)?;
        let id = header["id"]
            .as_u64()
            .ok_or_else(|| Error::local("missing engine client ID"))?;
        Ok(Client {
            resource: Resource::new(1, id),
        })
    }
}
struct RequestData {
    method: Method,
    url: String,
    headers: HeaderMap,
    body: Vec<u8>,
    multipart: Option<multipart::Form>,
    timeout: Option<Duration>,
}
pub struct RequestBuilder {
    client: Client,
    request: Result<RequestData, Error>,
}
impl RequestBuilder {
    fn change(mut self, f: impl FnOnce(&mut RequestData) -> Result<(), Error>) -> Self {
        if let Ok(request) = &mut self.request
            && let Err(error) = f(request)
        {
            self.request = Err(error);
        }
        self
    }
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        K::Error: fmt::Display,
        V: TryInto<HeaderValue>,
        V::Error: fmt::Display,
    {
        self.change(|request| {
            request.headers.append(
                key.try_into().map_err(Error::local)?,
                value.try_into().map_err(Error::local)?,
            );
            Ok(())
        })
    }
    pub fn headers(self, headers: HeaderMap) -> Self {
        self.change(|r| {
            r.headers.extend(headers);
            Ok(())
        })
    }
    pub fn bearer_auth(self, token: impl fmt::Display) -> Self {
        self.header(http::header::AUTHORIZATION, format!("Bearer {token}"))
    }
    pub fn timeout(self, value: Duration) -> Self {
        self.change(|r| {
            r.timeout = Some(value);
            Ok(())
        })
    }
    pub fn body(self, bytes: impl Into<Vec<u8>>) -> Self {
        self.change(|r| {
            r.multipart = None;
            r.body = bytes.into();
            Ok(())
        })
    }
    pub fn json<T: Serialize + ?Sized>(self, value: &T) -> Self {
        self.change(|r| {
            r.body = serde_json::to_vec(value).map_err(Error::local)?;
            r.multipart = None;
            r.headers
                .entry(http::header::CONTENT_TYPE)
                .or_insert(HeaderValue::from_static("application/json"));
            Ok(())
        })
    }
    pub fn query<T: Serialize + ?Sized>(self, value: &T) -> Self {
        self.change(|r| {
            let encoded = serde_urlencoded::to_string(value).map_err(Error::local)?;
            if !encoded.is_empty() {
                let fragment = r.url.find('#').unwrap_or(r.url.len());
                let separator = if r.url[..fragment].contains('?') {
                    '&'
                } else {
                    '?'
                };
                r.url.insert_str(fragment, &format!("{separator}{encoded}"));
            }
            Ok(())
        })
    }
    pub fn multipart(self, form: multipart::Form) -> Self {
        self.change(|r| {
            r.body.clear();
            r.multipart = Some(form);
            Ok(())
        })
    }
    pub async fn send(self) -> Result<Response, Error> {
        let request = self.request?;
        let (multipart, _callbacks) = match request.multipart {
            Some(form) => form.encode()?,
            None => (Value::Null, Vec::new()),
        };
        let headers: Vec<_> = request
            .headers
            .iter()
            .map(|(k, v)| json!([k.as_str(), v.as_bytes()]))
            .collect();
        let (header, _) = crate::engine_wire::command(json!({"op":"http_send", "multipart": multipart, "client": self.client.resource.id(), "method": request.method.as_str(), "url": request.url, "headers": headers, "timeout_ms": request.timeout.map(millis)}), &request.body).await.map_err(Error::local)?;
        let header = checked(header)?;
        let id = header["id"]
            .as_u64()
            .ok_or_else(|| Error::local("missing response ID"))?;
        let resource = Resource::new(2, id);
        let status = StatusCode::from_u16(
            header["status"]
                .as_u64()
                .and_then(|v| v.try_into().ok())
                .ok_or_else(|| Error::local("invalid response status"))?,
        )
        .map_err(Error::local)?;
        let mut headers = HeaderMap::new();
        for pair in header["headers"]
            .as_array()
            .ok_or_else(|| Error::local("missing response headers"))?
        {
            let key = HeaderName::from_bytes(
                pair[0]
                    .as_str()
                    .ok_or_else(|| Error::local("invalid header name"))?
                    .as_bytes(),
            )
            .map_err(Error::local)?;
            let value: Vec<u8> = serde_json::from_value(pair[1].clone()).map_err(Error::local)?;
            headers.append(key, HeaderValue::from_bytes(&value).map_err(Error::local)?);
        }
        Ok(Response {
            resource,
            status,
            headers,
            url: header["url"].as_str().unwrap_or_default().to_owned(),
            length: header["content_length"].as_u64(),
            eof: false,
        })
    }
}

pub struct Response {
    resource: Resource,
    status: StatusCode,
    headers: HeaderMap,
    url: String,
    length: Option<u64>,
    eof: bool,
}
impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
impl Response {
    pub fn status(&self) -> StatusCode {
        self.status
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn content_length(&self) -> Option<u64> {
        self.length
    }
    pub fn error_for_status(self) -> Result<Self, Error> {
        if self.status.is_client_error() || self.status.is_server_error() {
            Err(Error {
                message: format!("HTTP status {} for {}", self.status, self.url),
                timeout: false,
                connect: false,
                status: Some(self.status),
            })
        } else {
            Ok(self)
        }
    }
    pub async fn chunk(&mut self) -> Result<Option<Bytes>, Error> {
        if self.eof {
            return Ok(None);
        }
        let (header, body) = crate::engine_wire::command(
            json!({"op":"http_chunk","response":self.resource.id()}),
            &[],
        )
        .await
        .map_err(Error::local)?;
        let header = checked(header)?;
        self.eof = header["eof"] == true;
        Ok((!self.eof).then(|| Bytes::from(body)))
    }
    pub fn bytes_stream(self) -> impl futures_util::Stream<Item = Result<Bytes, Error>> + Send {
        futures_util::stream::try_unfold(self, |mut response| async move {
            Ok(response.chunk().await?.map(|bytes| (bytes, response)))
        })
    }
    pub async fn bytes(mut self) -> Result<Bytes, Error> {
        let mut bytes = Vec::new();
        while let Some(chunk) = self.chunk().await? {
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes.into())
    }
    pub async fn text(self) -> Result<String, Error> {
        let (header, body) = crate::engine_wire::command(
            json!({"op":"http_text","response":self.resource.id()}),
            &[],
        )
        .await
        .map_err(Error::local)?;
        checked(header)?;
        String::from_utf8(body).map_err(Error::local)
    }
    pub async fn json<T: DeserializeOwned>(self) -> Result<T, Error> {
        serde_json::from_slice(&self.bytes().await?).map_err(Error::local)
    }
}
