//! OIDC implementation and crypto types never cross the engine boundary.
use crate::operations::{FutureBytes, encode, id};
use openidconnect::{
    AccessTokenHash, AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

pub const CLIENT: u32 = 20;
type Client = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;
struct Provider {
    config: Value,
    client: tokio::sync::OnceCell<(Client, Http)>,
}
static CLIENTS: OnceLock<Mutex<HashMap<u64, Arc<Provider>>>> = OnceLock::new();
fn clients() -> &'static Mutex<HashMap<u64, Arc<Provider>>> {
    CLIENTS.get_or_init(Default::default)
}
pub fn release(id: u64) {
    let provider = clients().lock().unwrap().remove(&id);
    drop(provider);
}
fn field<'a>(config: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    config[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("configuration")
}
fn endpoint(url: &reqwest::Url, loopback: bool) -> bool {
    (url.scheme() == "https"
        || (loopback
            && url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
}
pub fn new(config: Value) -> Result<Vec<u8>, String> {
    let id = id();
    clients().lock().unwrap().insert(
        id,
        Arc::new(Provider {
            config,
            client: tokio::sync::OnceCell::new(),
        }),
    );
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
pub fn operation(command: Value) -> Result<FutureBytes, String> {
    let provider = command["id"]
        .as_u64()
        .and_then(|id| clients().lock().unwrap().get(&id).cloned())
        .ok_or("unknown OIDC client")?;
    Ok(Box::pin(async move {
        match execute(&provider, command).await {
            Ok(value) => encode(value, &[]),
            Err(error) => encode(json!({"ok":false,"error":error}), &[]),
        }
    }))
}
async fn discover(config: &Value) -> Result<(Client, Http), &'static str> {
    let loopback = config["allow_loopback_http"].as_bool().unwrap_or(false);
    let issuer = IssuerUrl::new(field(config, "issuer")?.into()).map_err(|_| "configuration")?;
    let redirect =
        RedirectUrl::new(field(config, "redirect_uri")?.into()).map_err(|_| "configuration")?;
    if !endpoint(issuer.url(), loopback) || !endpoint(redirect.url(), loopback) {
        return Err("unsafe_endpoint");
    }
    let client_id = ClientId::new(field(config, "client_id")?.into());
    let secret = ClientSecret::new(field(config, "client_secret")?.into());
    let http = Http {
        client: reqwest::Client::builder()
            .https_only(!loopback)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| "discovery")?,
        loopback,
    };
    let metadata = CoreProviderMetadata::discover_async(issuer, &http)
        .await
        .map_err(|_| "discovery")?;
    for url in [
        Some(metadata.authorization_endpoint().url()),
        metadata.token_endpoint().map(|u| u.url()),
        Some(metadata.jwks_uri().url()),
    ]
    .into_iter()
    .flatten()
    {
        if !endpoint(url, loopback) {
            return Err("unsafe_endpoint");
        }
    }
    let client = CoreClient::from_provider_metadata(metadata, client_id, Some(secret))
        .set_redirect_uri(redirect);
    Ok((client, http))
}
async fn execute(provider: &Provider, command: Value) -> Result<Value, &'static str> {
    if command["op"] == "oidc_discover" {
        provider
            .client
            .get_or_try_init(|| discover(&provider.config))
            .await?;
        return Ok(json!({"ok":true}));
    }
    let (client, http) = provider.client.get().ok_or("configuration")?;
    match command["op"].as_str() {
        Some("oidc_begin") => {
            let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
            let (url, state, nonce) = client
                .authorize_url(
                    AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                    CsrfToken::new_random,
                    Nonce::new_random,
                )
                .set_pkce_challenge(challenge)
                .url();
            Ok(
                json!({"ok":true,"url":url.as_str(),"state":state.secret(),"nonce":nonce.secret(),"verifier":verifier.secret()}),
            )
        }
        Some("oidc_finish") => {
            let denied = |_| "unauthorized";
            let code = field(&command, "code").map_err(denied)?;
            let nonce = field(&command, "nonce").map_err(denied)?;
            let pkce = field(&command, "verifier").map_err(denied)?;
            let response = client
                .exchange_code(AuthorizationCode::new(code.into()))
                .map_err(|_| "unauthorized")?
                .set_pkce_verifier(PkceCodeVerifier::new(pkce.into()))
                .request_async(http)
                .await
                .map_err(|_| "unauthorized")?;
            let token = response.id_token().ok_or("unauthorized")?;
            let verifier = client.id_token_verifier();
            let claims = token
                .claims(&verifier, &Nonce::new(nonce.into()))
                .map_err(|_| "unauthorized")?;
            if let Some(expected) = claims.access_token_hash() {
                let actual = AccessTokenHash::from_token(
                    response.access_token(),
                    token.signing_alg().map_err(|_| "unauthorized")?,
                    token.signing_key(&verifier).map_err(|_| "unauthorized")?,
                )
                .map_err(|_| "unauthorized")?;
                if actual != *expected {
                    return Err("unauthorized");
                }
            }
            Ok(json!({"ok":true,"subject":claims.subject().as_str()}))
        }
        _ => Err("configuration"),
    }
}
struct Http {
    client: reqwest::Client,
    loopback: bool,
}
impl<'a> openidconnect::AsyncHttpClient<'a> for Http {
    type Error = std::io::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<openidconnect::HttpResponse, Self::Error>> + Send + 'a>>;
    fn call(&'a self, request: openidconnect::HttpRequest) -> Self::Future {
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let url = reqwest::Url::parse(&parts.uri.to_string())
                .map_err(|_| std::io::Error::other("invalid OIDC endpoint"))?;
            // Also enforce policy on discovery's JWKS request, before any I/O.
            if !endpoint(&url, self.loopback) {
                return Err(std::io::Error::other("unsafe OIDC endpoint"));
            }
            let response = self
                .client
                .request(parts.method, url)
                .headers(parts.headers)
                .body(body)
                .send()
                .await
                .map_err(|_| std::io::Error::other("OIDC HTTP failure"))?;
            let status = response.status();
            let headers = response.headers().clone();
            let body = response
                .bytes()
                .await
                .map_err(|_| std::io::Error::other("OIDC HTTP failure"))?
                .to_vec();
            let mut result = openidconnect::HttpResponse::new(body);
            *result.status_mut() = status;
            *result.headers_mut() = headers;
            Ok(result)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::{Context, Poll, Waker};

    #[tokio::test]
    async fn outgoing_requests_enforce_endpoint_policy_before_io() {
        for loopback in [false, true] {
            let http = Http {
                client: reqwest::Client::new(),
                loopback,
            };
            for url in [
                "http://provider.invalid/jwks",
                "https://user:secret@provider.invalid/jwks",
            ] {
                let request = openidconnect::http::Request::builder()
                    .uri(url)
                    .body(Vec::new())
                    .unwrap();
                let error = openidconnect::AsyncHttpClient::call(&http, request)
                    .await
                    .unwrap_err();
                assert_eq!(error.to_string(), "unsafe OIDC endpoint");
            }
            for url in [
                "http://localhost/jwks",
                "http://127.0.0.1/jwks",
                "http://[::1]/jwks",
            ] {
                assert_eq!(endpoint(&url.parse().unwrap(), loopback), loopback);
            }
            assert!(endpoint(
                &"https://provider.invalid/jwks".parse().unwrap(),
                loopback
            ));
        }
    }

    #[tokio::test]
    async fn cancelled_discovery_releases_provider_after_host_handle_is_dropped() {
        // The listener never answers, so discovery must suspend before completion.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bytes = new(
            json!({"issuer":format!("http://{}", listener.local_addr().unwrap()),
            "client_id":"test", "client_secret":"test-secret",
            "redirect_uri":"https://service.example/callback", "allow_loopback_http":true}),
        )
        .unwrap();
        let (reply, _) = crate::operations::decode(&bytes).unwrap();
        let id = reply["id"].as_u64().unwrap();
        let weak = Arc::downgrade(clients().lock().unwrap().get(&id).unwrap());
        let mut operation = operation(json!({"op":"oidc_discover", "id":id})).unwrap();
        assert!(matches!(
            operation
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        release(id);
        assert!(
            weak.upgrade().is_some(),
            "in-flight discovery retains its provider"
        );
        drop(operation);
        assert!(
            weak.upgrade().is_none(),
            "cancelling discovery must not leak the provider"
        );
    }
}
