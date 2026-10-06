use simple_server::oidc::{Client, Config, Error};

#[simple_server::main]
async fn main() {
    // An actual call into the prebuilt implementation, with no external provider
    // or host-side OIDC/crypto dependencies needed by this standalone consumer.
    let error = Client::discover(Config {
        issuer: "not a URL".into(),
        client_id: "fixture".into(),
        client_secret: "fixture-secret".into(),
        redirect_uri: "https://service.example/callback".into(),
        allow_loopback_http: false,
    })
    .await
    .unwrap_err();
    assert_eq!(error, Error::Configuration);
    println!("engine OIDC consumer passed");
}
