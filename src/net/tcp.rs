use crate::net::{Socket, *};
use socket2::{Protocol, Type};
use std::net::Shutdown;

#[cfg(feature = "runtime")]
use crate::{buf::*, *};

/// A TCP socket server, listening for connections.
///
/// You can accept a new connection by using the [`accept`](`TcpListener::accept`)
/// method.
///
/// # Examples
///
/// ```
/// use compio::net::{TcpListener, TcpStream};
/// use socket2::SockAddr;
/// use std::net::SocketAddr;
///
/// let addr: SockAddr = "127.0.0.1:2345".parse::<SocketAddr>().unwrap().into();
///
/// let listener = TcpListener::bind(&addr).unwrap();
///
/// compio::task::block_on(async move {
///     let tx_fut = TcpStream::connect(&addr);
///
///     let rx_fut = listener.accept();
///
///     let (tx, (rx, _)) = futures_util::try_join!(tx_fut, rx_fut).unwrap();
///
///     tx.send("test").await.0.unwrap();
///
///     let (_, buf) = rx.recv(Vec::with_capacity(4)).await;
///
///     assert_eq!(buf, b"test");
/// });
/// ```
pub struct TcpListener {
    inner: Socket,
}

impl TcpListener {
    /// Creates a new `TcpListener`, which will be bound to the specified address.
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port
    /// to this listener.
    pub fn bind(addr: impl ToSockAddrs) -> io::Result<Self> {
        super::each_addr(addr, |addr| {
            let socket = Socket::bind(&addr, Type::STREAM, Some(Protocol::TCP))?;
            socket.listen(128)?;
            Ok(Self { inner: socket })
        })
    }

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding [`TcpStream`] and the remote peer's
    /// address will be returned.
    #[cfg(feature = "runtime")]
    pub async fn accept(&self) -> io::Result<(TcpStream, SockAddr)> {
        let (socket, addr) = self.inner.accept().await?;
        let stream = TcpStream { inner: socket };
        Ok((stream, addr))
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to
    /// figure out which port was actually bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    /// use compio::net::TcpListener;
    /// use socket2::SockAddr;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    ///
    /// let addr = listener.local_addr().expect("Couldn't get local address");
    /// assert_eq!(addr, SockAddr::from(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080))));
    /// ```
    pub fn local_addr(&self) -> io::Result<SockAddr> {
        self.inner.local_addr()
    }
}

impl_raw_fd!(TcpListener, inner);

/// A TCP stream between a local and a remote socket.
///
/// A TCP stream can either be created by connecting to an endpoint, via the
/// `connect` method, or by accepting a connection from a listener.
///
/// # Examples
///
/// ```no_run
/// use compio::net::TcpStream;
/// use std::net::SocketAddr;
///
/// compio::task::block_on(async {
///     // Connect to a peer
///     let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
///
///     // Write some data.
///     let (result, _) = stream.send("hello world!").await;
///     result.unwrap();
/// })
/// ```
pub struct TcpStream {
    inner: Socket,
}

impl TcpStream {
    /// Opens a TCP connection to a remote host.
    #[cfg(feature = "runtime")]
    pub async fn connect(addr: impl ToSockAddrs) -> io::Result<Self> {
        super::each_addr_async(addr, |addr| async move {
            let mut socket_addr = addr.as_socket().unwrap();
            socket_addr.set_port(0);
            let bind_addr = SockAddr::from(socket_addr);
            let socket = Socket::bind(&bind_addr, Type::STREAM, Some(Protocol::TCP))?;
            socket.connect_async(&addr).await?;
            Ok(Self { inner: socket })
        })
        .await
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> io::Result<SockAddr> {
        self.inner.peer_addr()
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> io::Result<SockAddr> {
        self.inner.local_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O on the specified
    /// portions to return immediately with an appropriate value (see the
    /// documentation of [`Shutdown`]).
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }

    /// Receives a packet of data from the socket into the buffer, returning the original buffer and
    /// quantity of data received.
    #[cfg(feature = "runtime")]
    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.recv(buffer).await
    }

    /// Receives a packet of data from the socket into the buffer, returning the original buffer and
    /// quantity of data received.
    #[cfg(feature = "runtime")]
    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.recv_vectored(buffer).await
    }

    /// Sends some data to the socket from the buffer, returning the original buffer and
    /// quantity of data sent.
    #[cfg(feature = "runtime")]
    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.send(buffer).await
    }

    /// Sends some data to the socket from the buffer, returning the original buffer and
    /// quantity of data sent.
    #[cfg(feature = "runtime")]
    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.send_vectored(buffer).await
    }
}

impl_raw_fd!(TcpStream, inner);
