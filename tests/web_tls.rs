#![cfg(feature = "tls")]

use simple_server::{
    lifecycle::Shutdown,
    web::{
        Router,
        tls::{self, TlsConfig},
    },
};

#[tokio::test]
async fn invalid_pem_is_rejected() {
    assert!(
        TlsConfig::from_pem(b"not a certificate".to_vec(), b"not a key".to_vec())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn previously_requested_shutdown_does_not_start_tls_server() {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let config = TlsConfig::from_pem(
        certificate.cert.pem().into_bytes(),
        certificate.key_pair.serialize_pem().into_bytes(),
    )
    .await
    .unwrap();
    let listener = simple_server::http::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = Shutdown::new();
    shutdown.request();

    tls::serve(listener, Router::new(), config, shutdown)
        .await
        .unwrap();
    let rebound = simple_server::http::bind(address).await.unwrap();
    drop(rebound);
}
