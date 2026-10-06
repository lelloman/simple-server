//! Engine-owned OpenID Connect authorization-code flows with PKCE.
//!
//! The application must bind `state` to the browser, persist the nonce and PKCE
//! verifier securely, and atomically consume them before calling [`Client::finish`](crate::oidc::Client::finish).
//! Sessions, account mapping and permissions remain application-owned.
use serde_json::{Value, json};
use simple_server_sys::Resource;
use std::fmt;

/// Trusted provider/client configuration. Debug deliberately omits credentials.
pub struct Config {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    /// Permit HTTP only for localhost, 127.0.0.1 and `[::1]`. Default to false.
    pub allow_loopback_http: bool,
}
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config").finish_non_exhaustive()
    }
}

/// Errors contain no provider response, token, authorization code or secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    UnsafeEndpoint,
    Discovery,
    Unauthorized,
    Engine,
}
impl Error {
    pub fn message(self) -> &'static str {
        match self {
            Self::Configuration => "invalid OIDC configuration",
            Self::UnsafeEndpoint => "unsafe OIDC endpoint",
            Self::Discovery => "OIDC discovery failed",
            Self::Unauthorized => "OIDC authentication failed",
            Self::Engine => "OIDC engine failure",
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
impl std::error::Error for Error {}

/// Single-use flow material. Treat the verifier and nonce as secrets.
pub struct Authorization {
    pub url: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
}
impl fmt::Debug for Authorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Authorization").finish_non_exhaustive()
    }
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
fn checked(value: Value) -> Result<Value, Error> {
    if value["ok"] == true {
        Ok(value)
    } else {
        Err(match value["error"].as_str() {
            Some("configuration") => Error::Configuration,
            Some("unsafe_endpoint") => Error::UnsafeEndpoint,
            Some("discovery") => Error::Discovery,
            Some("unauthorized") => Error::Unauthorized,
            _ => Error::Engine,
        })
    }
}
fn string(value: &Value, key: &str) -> Result<String, Error> {
    value[key].as_str().map(str::to_owned).ok_or(Error::Engine)
}
impl Client {
    /// Discover provider metadata and keys. Redirects are disabled; each HTTP
    /// request has a ten-second timeout. Only HTTPS is allowed unless explicitly
    /// opting into loopback HTTP. The issuer must match discovery metadata.
    pub async fn discover(config: Config) -> Result<Self, Error> {
        let command = json!({"op":"oidc_new", "issuer":config.issuer,
            "client_id":config.client_id, "client_secret":config.client_secret,
            "redirect_uri":config.redirect_uri, "allow_loopback_http":config.allow_loopback_http});
        let bytes = simple_server_sys::resource_new(
            &serde_json::to_vec(&command).map_err(|_| Error::Engine)?,
        )
        .map_err(|_| Error::Engine)?;
        let (value, _) = crate::engine_wire::decode(&bytes).map_err(|_| Error::Engine)?;
        let value = checked(value)?;
        // Allocate the resource before awaiting discovery so cancellation cannot
        // strand a newly created client in the native registry.
        let client = Self {
            resource: Resource::new(20, value["id"].as_u64().ok_or(Error::Engine)?),
        };
        client.command(json!({"op":"oidc_discover"})).await?;
        Ok(client)
    }
    async fn command(&self, mut value: Value) -> Result<Value, Error> {
        value["id"] = self.resource.id().into();
        let (value, _) = crate::engine_wire::command(value, &[])
            .await
            .map_err(|_| Error::Engine)?;
        checked(value)
    }
    /// Generate fresh state, nonce and an S256 PKCE challenge/verifier.
    pub async fn begin(&self) -> Result<Authorization, Error> {
        let value = self.command(json!({"op":"oidc_begin"})).await?;
        Ok(Authorization {
            url: string(&value, "url")?,
            state: string(&value, "state")?,
            nonce: string(&value, "nonce")?,
            pkce_verifier: string(&value, "verifier")?,
        })
    }
    /// Exchange a code, verify the ID token and optional access-token hash, and
    /// return its verified subject. The caller must already have checked browser
    /// binding/state and atomically consumed the stored flow. A returned subject
    /// identifies an account only within this client's configured issuer.
    pub async fn finish(
        &self,
        code: &str,
        nonce: &str,
        pkce_verifier: &str,
    ) -> Result<String, Error> {
        let value = self
            .command(json!({"op":"oidc_finish", "code":code,
            "nonce":nonce,"verifier":pkce_verifier}))
            .await?;
        string(&value, "subject")
    }
}
