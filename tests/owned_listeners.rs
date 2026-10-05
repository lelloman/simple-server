#![cfg(all(feature = "http", feature = "lifecycle"))]

use simple_server::net::{TcpListener, ToSocketAddrs};
use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

async fn bind_and_release(address: impl ToSocketAddrs) {
    let listener = simple_server::http::bind(address).await.unwrap();
    let bound = listener.local_addr().unwrap();
    assert_ne!(bound.port(), 0);
    drop(listener);
    drop(TcpListener::bind(bound).await.unwrap());
}

#[tokio::test]
async fn supported_addresses_resolve_and_bind_without_backend_types() {
    bind_and_release("127.0.0.1:0").await;
    bind_and_release("127.0.0.1:0".to_owned()).await;
    bind_and_release(&"127.0.0.1:0".to_owned()).await;
    bind_and_release(("localhost", 0)).await;
    bind_and_release(("localhost".to_owned(), 0)).await;
    bind_and_release((Ipv4Addr::LOCALHOST, 0)).await;
    bind_and_release((IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await;
    let v4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    bind_and_release(v4).await;
    bind_and_release(SocketAddr::V4(v4)).await;
    bind_and_release([SocketAddr::V4(v4)]).await;
    bind_and_release(&[SocketAddr::V4(v4)][..]).await;
    bind_and_release(vec![SocketAddr::V4(v4)]).await;

    // Check IPv6 address metadata without requiring IPv6-enabled networking.
    let v6 = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 42, 3, 7);
    assert_eq!(v6.to_socket_addrs().await.unwrap(), [SocketAddr::V6(v6)]);
    assert_eq!(
        (Ipv6Addr::LOCALHOST, 42).to_socket_addrs().await.unwrap(),
        [SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 42)]
    );
    assert_eq!(
        "[::1]:42".to_socket_addrs().await.unwrap(),
        [SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 42)]
    );
}

#[tokio::test]
async fn fallback_last_error_and_empty_list_preserve_bind_semantics() {
    let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = occupied.local_addr().unwrap();
    bind_and_release([address, "127.0.0.1:0".parse().unwrap()]).await;
    let error = TcpListener::bind([address, address]).await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
    assert!(error.raw_os_error().is_some());
    assert_eq!(
        TcpListener::bind(Vec::<SocketAddr>::new())
            .await
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        TcpListener::bind("no-port").await.unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[tokio::test]
async fn standard_listener_round_trip_keeps_port_and_socket_options() {
    let original = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    original.set_ttl(37).unwrap();
    let address = original.local_addr().unwrap();
    let listener = TcpListener::from_std(original).unwrap();
    assert_eq!(listener.local_addr().unwrap(), address);
    let returned = listener.into_std().unwrap();
    assert_eq!(returned.ttl().unwrap(), 37);
    assert_eq!(
        returned.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(returned);
    bind_and_release(address).await;
}

#[tokio::test]
async fn custom_resolver_errors_are_preserved() {
    struct Unavailable;
    impl ToSocketAddrs for Unavailable {
        async fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "resolver denied",
            ))
        }
    }
    let error = TcpListener::bind(Unavailable).await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(error.to_string(), "resolver denied");
}

#[cfg(all(unix, feature = "unix-http"))]
#[tokio::test]
async fn unix_listener_round_trip_keeps_path_and_does_not_unlink_it() {
    use simple_server::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owned.sock");
    let original = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let listener = UnixListener::from_std(original).unwrap();
    assert_eq!(
        listener.local_addr().unwrap().as_pathname(),
        Some(path.as_path())
    );
    let returned = listener.into_std().unwrap();
    assert_eq!(
        returned.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(returned);
    assert!(path.exists());
    assert_eq!(
        UnixListener::bind(&path).unwrap_err().kind(),
        io::ErrorKind::AddrInUse
    );
}
