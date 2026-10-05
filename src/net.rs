//! Owned listener and address contracts for server transports.
//!
//! The current source backend requires an active Tokio I/O runtime. Backend
//! types stay private so serving callers do not depend on its networking API.
use std::{
    future::Future,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

/// Asynchronously resolve an address into candidates, in connection order.
///
/// Supports strings, host/port tuples, IP/port tuples, socket addresses, slices,
/// arrays, vectors and references. Custom resolvers can implement this contract
/// without depending on a particular runtime. Resolution must not block the
/// executor thread. An empty result is rejected by [`TcpListener::bind`].
pub trait ToSocketAddrs: Sync {
    fn to_socket_addrs(&self) -> impl Future<Output = io::Result<Vec<SocketAddr>>> + Send;
}

macro_rules! address {
    ($($ty:ty),+ $(,)?) => {$ (
        impl ToSocketAddrs for $ty {
            async fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
                Ok(tokio::net::lookup_host(self).await?.collect())
            }
        }
    )+};
}
address!(
    str,
    String,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    (IpAddr, u16),
    (Ipv4Addr, u16),
    (Ipv6Addr, u16),
    (&str, u16),
    (String, u16)
);

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    fn to_socket_addrs(&self) -> impl Future<Output = io::Result<Vec<SocketAddr>>> + Send {
        T::to_socket_addrs(self)
    }
}
impl ToSocketAddrs for [SocketAddr] {
    async fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        Ok(self.to_vec())
    }
}
impl<const N: usize> ToSocketAddrs for [SocketAddr; N] {
    async fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        Ok(self.to_vec())
    }
}
impl ToSocketAddrs for Vec<SocketAddr> {
    async fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        Ok(self.clone())
    }
}

/// An owned listener for the HTTP and TLS serving adapters.
///
/// Binding tries resolved addresses in order, returning the first success or
/// the last error. Dropping an unused listener closes it and releases the port.
#[derive(Debug)]
pub struct TcpListener {
    pub(crate) inner: tokio::net::TcpListener,
}
impl TcpListener {
    pub async fn bind(address: impl ToSocketAddrs) -> io::Result<Self> {
        let addresses = address.to_socket_addrs().await?;
        tokio::net::TcpListener::bind(addresses.as_slice())
            .await
            .map(|inner| Self { inner })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    /// Take ownership of a standard listener, enabling nonblocking mode before
    /// registering it with the current I/O runtime. Socket options are retained.
    pub fn from_std(listener: std::net::TcpListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        tokio::net::TcpListener::from_std(listener).map(|inner| Self { inner })
    }

    /// Return the underlying socket in nonblocking mode, without closing it.
    pub fn into_std(self) -> io::Result<std::net::TcpListener> {
        self.inner.into_std()
    }
}

/// An owned Unix listener. Binding and dropping never unlink its socket path.
#[cfg(all(unix, feature = "unix-http"))]
#[derive(Debug)]
pub struct UnixListener {
    pub(crate) inner: tokio::net::UnixListener,
}
#[cfg(all(unix, feature = "unix-http"))]
impl UnixListener {
    pub fn bind(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        tokio::net::UnixListener::bind(path).map(|inner| Self { inner })
    }

    pub fn local_addr(&self) -> io::Result<std::os::unix::net::SocketAddr> {
        self.inner.local_addr().map(Into::into)
    }

    /// Enable nonblocking mode and register an application-configured socket.
    pub fn from_std(listener: std::os::unix::net::UnixListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        tokio::net::UnixListener::from_std(listener).map(|inner| Self { inner })
    }

    pub fn into_std(self) -> io::Result<std::os::unix::net::UnixListener> {
        self.inner.into_std()
    }
}
