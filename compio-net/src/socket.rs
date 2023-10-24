use std::io;

#[cfg(all(feature = "runtime", target_os = "linux"))]
use compio_driver::ring_mapped_buffers::{RawRingMappedBuffers, RingMappedBuffer};
use compio_driver::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use socket2::{Domain, Protocol, SockAddr, Socket as Socket2, Type};
#[cfg(feature = "runtime")]
use {
    compio_buf::{buf_try, BufResult, IntoInner, IoBuf, IoBufMut, IoVectoredBuf, IoVectoredBufMut},
    compio_driver::op::{
        Accept, BufResultExt, Connect, Recv, RecvFrom, RecvFromVectored, RecvResultExt,
        RecvVectored, RecvWithRegisterBuffers, Send, SendTo, SendToVectored, SendVectored,
    },
    compio_io::{AsyncRead, AsyncWrite},
    compio_runtime::{submit, Attachable, Attacher},
};

#[derive(Debug)]
pub struct Socket {
    socket: Socket2,
    #[cfg(feature = "runtime")]
    attacher: Attacher,
}

impl Socket {
    pub fn from_socket2(socket: Socket2) -> Self {
        Self {
            socket,
            #[cfg(feature = "runtime")]
            attacher: Attacher::new(),
        }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        let socket = self.socket.try_clone()?;
        Ok(Self {
            #[cfg(feature = "runtime")]
            attacher: self.attacher.try_clone(&socket)?,
            socket,
        })
    }

    pub fn peer_addr(&self) -> io::Result<SockAddr> {
        self.socket.peer_addr()
    }

    pub fn local_addr(&self) -> io::Result<SockAddr> {
        self.socket.local_addr()
    }

    pub fn new(domain: Domain, ty: Type, protocol: Option<Protocol>) -> io::Result<Self> {
        let socket = Socket2::new(domain, ty, protocol)?;
        // On Linux we use blocking socket
        // Newer kernels have the patch that allows to arm io_uring poll mechanism for
        // non blocking socket when there is no connections in listen queue
        //
        // https://patchwork.kernel.org/project/linux-block/patch/f999615b-205c-49b7-b272-c4e42e45e09d@kernel.dk/#22949861
        if cfg!(all(
            unix,
            not(all(target_os = "linux", feature = "io-uring"))
        )) {
            socket.set_nonblocking(true)?;
        }
        Ok(Self::from_socket2(socket))
    }

    pub fn bind(addr: &SockAddr, ty: Type, protocol: Option<Protocol>) -> io::Result<Self> {
        let socket = Self::new(addr.domain(), ty, protocol)?;
        socket.socket.bind(addr)?;
        Ok(socket)
    }

    pub fn listen(&self, backlog: i32) -> io::Result<()> {
        self.socket.listen(backlog)
    }

    pub fn connect(&self, addr: &SockAddr) -> io::Result<()> {
        self.socket.connect(addr)
    }

    #[cfg(feature = "runtime")]
    pub async fn connect_async(&self, addr: &SockAddr) -> io::Result<()> {
        self.attach()?;
        let op = Connect::new(self.as_raw_fd(), addr.clone());
        let BufResult(res, _op) = submit(op).await;
        #[cfg(windows)]
        {
            res?;
            _op.update_context()?;
            Ok(())
        }
        #[cfg(unix)]
        {
            res.map(|_| ())
        }
    }

    #[cfg(all(feature = "runtime", unix))]
    pub async fn accept(&self) -> io::Result<(Self, SockAddr)> {
        self.attach()?;
        let op = Accept::new(self.as_raw_fd());
        let BufResult(res, op) = submit(op).await;
        let accept_sock = unsafe { Socket2::from_raw_fd(res? as _) };
        if cfg!(all(
            unix,
            not(all(target_os = "linux", feature = "io-uring"))
        )) {
            accept_sock.set_nonblocking(true)?;
        }
        let accept_sock = Self::from_socket2(accept_sock);
        let addr = op.into_addr();
        Ok((accept_sock, addr))
    }

    #[cfg(all(feature = "runtime", windows))]
    pub async fn accept(&self) -> io::Result<(Self, SockAddr)> {
        self.attach()?;
        let local_addr = self.local_addr()?;
        let accept_sock = Self::new(
            local_addr.domain(),
            self.socket.r#type()?,
            self.socket.protocol()?,
        )?;
        let op = Accept::new(self.as_raw_fd(), accept_sock.as_raw_fd() as _);
        let BufResult(res, op) = submit(op).await;
        res?;
        op.update_context()?;
        let addr = op.into_addr()?;
        Ok((accept_sock, addr))
    }

    #[cfg(all(feature = "runtime", target_os = "linux"))]
    pub async fn recv_with_ring_mapped_buffers(
        &self,
        buffers: RawRingMappedBuffers,
    ) -> BufResult<RingMappedBuffer, RawRingMappedBuffers> {
        let (_, buffers) = buf_try!(self.attach(), buffers);
        let op = RecvWithRegisterBuffers::new(self.as_raw_fd(), buffers);
        let BufResult(res, op) = submit(op).await;

        match res {
            Err(err) => BufResult(Err(err), op.into_inner()),
            Ok(n) => BufResult(op.buf(n), op.into_inner()),
        }
    }

    #[cfg(feature = "runtime")]
    pub async fn recv_from<T: IoBufMut>(&mut self, buffer: T) -> BufResult<(usize, SockAddr), T> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = RecvFrom::new(self.as_raw_fd(), buffer);
        submit(op).await.into_inner().map_addr().map_advanced()
    }

    #[cfg(feature = "runtime")]
    pub async fn recv_from_vectored<T: IoVectoredBufMut>(
        &mut self,
        buffer: T,
    ) -> BufResult<(usize, SockAddr), T> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = RecvFromVectored::new(self.as_raw_fd(), buffer);
        submit(op).await.into_inner().map_addr().map_advanced()
    }

    #[cfg(feature = "runtime")]
    pub async fn send_to<T: IoBuf>(&mut self, buffer: T, addr: &SockAddr) -> BufResult<usize, T> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = SendTo::new(self.as_raw_fd(), buffer, addr.clone());
        submit(op).await.into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn send_to_vectored<T: IoVectoredBuf>(
        &mut self,
        buffer: T,
        addr: &SockAddr,
    ) -> BufResult<usize, T> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = SendToVectored::new(self.as_raw_fd(), buffer, addr.clone());
        submit(op).await.into_inner()
    }
}

#[cfg(feature = "runtime")]
impl AsyncRead for Socket {
    async fn read<B: IoBufMut>(&mut self, buffer: B) -> BufResult<usize, B> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = Recv::new(self.as_raw_fd(), buffer);
        submit(op).await.into_inner().map_advanced()
    }

    async fn read_vectored<V: IoVectoredBufMut>(&mut self, buffer: V) -> BufResult<usize, V>
    where
        V: Unpin + 'static,
    {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = RecvVectored::new(self.as_raw_fd(), buffer);
        submit(op).await.into_inner().map_advanced()
    }
}

#[cfg(feature = "runtime")]
impl AsyncWrite for Socket {
    async fn write<T: IoBuf>(&mut self, buffer: T) -> BufResult<usize, T> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = Send::new(self.as_raw_fd(), buffer);
        submit(op).await.into_inner()
    }

    async fn write_vectored<T: IoVectoredBuf>(&mut self, buffer: T) -> BufResult<usize, T> {
        let ((), buffer) = buf_try!(self.attach(), buffer);
        let op = SendVectored::new(self.as_raw_fd(), buffer);
        submit(op).await.into_inner()
    }

    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> io::Result<()> {
        self.socket.shutdown(std::net::Shutdown::Write)
    }
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            socket: FromRawFd::from_raw_fd(fd),
            #[cfg(feature = "runtime")]
            attacher: compio_runtime::Attacher::new(),
        }
    }
}

impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> RawFd {
        self.socket.into_raw_fd()
    }
}

#[cfg(feature = "runtime")]
impl Attachable for Socket {
    fn attach(&self) -> io::Result<()> {
        self.attacher.attach(self)
    }

    fn is_attached(&self) -> bool {
        self.attacher.is_attached()
    }
}
