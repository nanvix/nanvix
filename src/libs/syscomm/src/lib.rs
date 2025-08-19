// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::anyhow::Result;
use ::log::error;
use ::mio::{
    net::{
        TcpListener,
        TcpStream,
        UnixListener,
        UnixStream,
    },
    Events,
    Interest,
    Poll,
    Registry,
    Token,
};
use ::std::{
    error::Error,
    fmt,
    fs,
    io::{
        self,
        ErrorKind,
        Read,
        Write,
    },
    time::{
        Duration,
        Instant,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Blocking sockets use a per-socket poll structure with only one entry with this token.
const BLOCKING_THREAD_TOKEN: Token = Token(0);

//==================================================================================================
// Imports
//==================================================================================================

/// An enum representing the type of a socket.
#[derive(Debug, Clone, Copy)]
pub enum SocketType {
    /// TCP socket.
    Tcp,
    /// Unix socket.
    Unix,
}

/// A structure representing an unbound socket.
pub struct Socket;

impl Socket {
    pub fn bind(typ: SocketType, addr: String) -> Result<SocketListener> {
        match typ {
            SocketType::Tcp => Ok(SocketListener::Tcp(TcpListener::bind(addr.parse()?)?)),
            SocketType::Unix => Ok(SocketListener::Unix {
                listener: UnixListener::bind(&addr)?,
                path: addr.clone(),
            }),
        }
    }
}

/// A struct representing a bound socket.
#[derive(Debug)]
pub enum SocketListener {
    Tcp(TcpListener),
    Unix {
        listener: UnixListener,
        path: String,
    },
}

impl ::std::str::FromStr for SocketType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(SocketType::Tcp),
            "unix" => Ok(SocketType::Unix),
            _ => Err("invalid socket type"),
        }
    }
}

impl SocketListener {
    /// Accepts a connection on a socket.
    pub fn accept(&self) -> Result<SocketStream, SocketError> {
        match self {
            SocketListener::Tcp(listener) => {
                let (stream, _sockaddr): (TcpStream, std::net::SocketAddr) = listener.accept()?;
                Ok(SocketStream::Tcp(stream))
            },
            SocketListener::Unix { listener, path: _ } => {
                let (stream, _sockaddr): (UnixStream, std::os::unix::net::SocketAddr) =
                    listener.accept()?;
                Ok(SocketStream::Unix(stream))
            },
        }
    }

    /// Accepts a connection on a socket with a timeout.
    ///
    /// Our SocketStream abstraction is backed by mio sockets, so it is non-blocking. This method
    /// offers a wrapper to accept a connection with a timeout by requiring the caller to provide
    /// a poll structure.
    ///
    /// This is different from the BlockingSocketStream structure, that uses an internal poll to
    /// receive in a blocking fashion.
    pub fn accept_timeout(
        &self,
        poll: &mut Poll,
        timeout: Duration,
    ) -> Result<SocketStream, SocketError> {
        let deadline: Instant = Instant::now() + timeout;
        let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

        // Accept in a loop to account for spurious wake-ups in the poll.
        loop {
            // Try to accept first without blocking.
            match self.accept() {
                Ok(conn) => return Ok(conn),
                // If it blocks, proceed to sleeping in a poll.
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {},
                // If we were inerrupted, retry immediately.
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                // If there was a transient error in the connection, retry immediately.
                Err(ref e) if e.kind() == ErrorKind::ConnectionAborted => continue,
                Err(e) => {
                    error!("error accepting connection (error={e:?})");
                    return Err(e);
                },
            }

            let now: Instant = Instant::now();
            if now >= deadline {
                let reason: String = "accept timed-out waiting for connection".to_string();
                error!("{reason}");
                return Err(io::Error::new(ErrorKind::TimedOut, reason).into());
            }

            // Wait until the listener becomes readable or timeout expires
            events.clear();
            poll.poll(&mut events, Some(deadline - now))?;
        }
    }
}

impl Drop for SocketListener {
    fn drop(&mut self) {
        match self {
            SocketListener::Tcp(_) => {},
            SocketListener::Unix { listener: _, path } => match fs::remove_file(path.clone()) {
                Ok(_) => {},
                Err(ref e) if e.kind() == ErrorKind::NotFound => {},
                Err(e) => error!("error removing UNIX socket (path={path}, error={e:?})"),
            },
        }
    }
}

impl mio::event::Source for SocketListener {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        match self {
            SocketListener::Tcp(listener) => listener.register(registry, token, interests),
            SocketListener::Unix { listener, path: _ } => {
                listener.register(registry, token, interests)
            },
        }
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        match self {
            SocketListener::Tcp(listener) => listener.reregister(registry, token, interests),
            SocketListener::Unix { listener, path: _ } => {
                listener.reregister(registry, token, interests)
            },
        }
    }

    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        match self {
            SocketListener::Tcp(listener) => listener.deregister(registry),
            SocketListener::Unix { listener, path: _ } => listener.deregister(registry),
        }
    }
}

/// A struct representing a socket stream.
#[derive(Debug)]
pub enum SocketStream {
    /// TCP socket stream.
    Tcp(TcpStream),
    /// Unix socket stream.
    Unix(UnixStream),
}

impl SocketStream {
    /// Creates a new socket stream.
    pub fn connect(typ: SocketType, addr: String) -> Result<SocketStream, io::Error> {
        match typ {
            SocketType::Tcp => {
                let stream: TcpStream = TcpStream::connect(addr.parse().map_err(|_| {
                    io::Error::new(ErrorKind::InvalidData, format!("invalid TCP address: {addr}"))
                })?)?;
                Ok(SocketStream::Tcp(stream))
            },
            SocketType::Unix => {
                let stream: UnixStream = UnixStream::connect(addr)?;
                Ok(SocketStream::Unix(stream))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Set the stream to blocking mode.
    ///
    /// We use non-blocking sockets, so to provide a blocking-like behaviour we extend each stream
    /// with its own poll structure that can be used to wait on data.
    ///
    /// We return a different struct, a BlockingSocketStream, to enforce type checks.
    ///
    /// # Returns
    ///
    /// A blocking socket stream based on the same underlying stream.
    ///
    pub fn set_blocking(self) -> io::Result<BlockingSocketStream> {
        match self {
            SocketStream::Tcp(mut stream) => {
                // Initialize Poll structure if it's not already set
                let poll_instance = Poll::new().map_err(|e| {
                    io::Error::other(format!("failed to create Poll instance (error={e:?})"))
                })?;

                poll_instance
                    .registry()
                    .register(
                        &mut stream,
                        BLOCKING_THREAD_TOKEN,
                        Interest::READABLE | Interest::WRITABLE,
                    )
                    .map_err(|e| {
                        io::Error::other(format!("failed to register thread to poll (error={e:?})"))
                    })?;

                Ok(BlockingSocketStream::Tcp(stream, poll_instance))
            },
            SocketStream::Unix(mut stream) => {
                let poll_instance = Poll::new().map_err(|e| {
                    io::Error::other(format!("failed to create Poll instance (error={e:?})"))
                })?;

                poll_instance
                    .registry()
                    .register(
                        &mut stream,
                        BLOCKING_THREAD_TOKEN,
                        Interest::READABLE | Interest::WRITABLE,
                    )
                    .map_err(|e| {
                        io::Error::other(format!("failed to register thread to poll (error={e:?})"))
                    })?;

                Ok(BlockingSocketStream::Unix(stream, poll_instance))
            },
        }
    }

    /// Writes data to a socket stream.
    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), SocketError> {
        let result = match self {
            SocketStream::Tcp(stream) => stream.write_all(buf),
            SocketStream::Unix(stream) => stream.write_all(buf),
        };

        match result {
            Ok(_) => Ok(()),
            Err(error) => Err(SocketError { error }),
        }
    }

    ///
    /// # Description
    ///
    /// Non-blocking read implementation.
    ///
    /// # Parameters
    ///
    /// A mutable buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read into the buffer.
    ///
    pub fn try_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            SocketStream::Tcp(stream) => stream.read(buf),
            SocketStream::Unix(stream) => stream.read(buf),
        }
    }

    ///
    /// # Description
    ///
    /// Non-blocking read exact implementation. This implementation returns partial reads and does
    /// no buffering. Callers must handle partial reads themselves and retry.
    ///
    /// # Parameters
    ///
    /// A mutable buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read into the buffer.
    ///
    pub fn try_read_exact(&mut self, buf: &mut [u8]) -> Result<usize, SocketError> {
        let mut total_read = 0;

        while total_read < buf.len() {
            match self.try_read(&mut buf[total_read..]) {
                Ok(0) => {
                    return Err(io::Error::new(ErrorKind::UnexpectedEof, "connection closed").into())
                },
                Ok(n) => total_read += n,
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Not ready yet — must wait for next Poll notification
                    break;
                },
                Err(e) => return Err(e.into()),
            }
        }

        // Return as much as we were able to read.
        Ok(total_read)
    }

    /// Gets the peer address of a socket stream.
    pub fn peer_addr(&self) -> Result<SocketAddr, SocketError> {
        match self {
            SocketStream::Tcp(stream) => match stream.peer_addr() {
                Ok(addr) => Ok(SocketAddr::Tcp(addr)),
                Err(error) => Err(SocketError { error }),
            },
            SocketStream::Unix(stream) => match stream.peer_addr() {
                Ok(addr) => Ok(SocketAddr::Unix(addr)),
                Err(error) => Err(SocketError { error }),
            },
        }
    }
}

impl mio::event::Source for SocketStream {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        match self {
            SocketStream::Tcp(stream) => stream.register(registry, token, interests),
            SocketStream::Unix(stream) => stream.register(registry, token, interests),
        }
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        match self {
            SocketStream::Tcp(stream) => stream.reregister(registry, token, interests),
            SocketStream::Unix(stream) => stream.reregister(registry, token, interests),
        }
    }

    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        match self {
            SocketStream::Tcp(stream) => stream.deregister(registry),
            SocketStream::Unix(stream) => stream.deregister(registry),
        }
    }
}

/// A struct representing a blocking socket stream.
#[derive(Debug)]
pub enum BlockingSocketStream {
    /// TCP socket stream.
    Tcp(TcpStream, Poll),
    /// Unix socket stream.
    Unix(UnixStream, Poll),
}

impl BlockingSocketStream {
    ///
    /// # Description
    ///
    /// Blocking read implementation.
    ///
    /// # Parameters
    ///
    /// A mutable buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read into the buffer.
    ///
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            BlockingSocketStream::Tcp(stream, poll) => {
                let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

                // Even after a poll wake-up the socket may still return WouldBlock.
                loop {
                    poll.poll(&mut events, None)?;
                    match stream.read(buf) {
                        Ok(n) => return Ok(n),
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Err(e) => return Err(e),
                    }
                }
            },
            BlockingSocketStream::Unix(stream, poll) => {
                let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

                // Even after a poll wake-up the socket may still return WouldBlock.
                loop {
                    poll.poll(&mut events, None)?;
                    match stream.read(buf) {
                        Ok(n) => return Ok(n),
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Err(e) => return Err(e),
                    }
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Blocking read exact implementation.
    ///
    /// # Parameters
    ///
    /// A mutable buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read into the buffer.
    ///
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), SocketError> {
        let mut num_read = 0;
        while num_read < buf.len() {
            match self.read(&mut buf[num_read..]) {
                Ok(0) => {
                    return Err(
                        io::Error::new(io::ErrorKind::UnexpectedEof, "End of file reached").into()
                    )
                },
                Ok(n) => num_read += n,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Writes data to a socket stream.
    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), SocketError> {
        let do_write_all = |stream: &mut dyn Write, poll: &mut Poll| -> Result<(), SocketError> {
            let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);
            loop {
                match stream.write_all(buf) {
                    Ok(()) => return Ok(()),
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {},
                    Err(e) => return Err(SocketError { error: e }),
                }

                poll.poll(&mut events, None)?;
            }
        };

        match self {
            BlockingSocketStream::Tcp(stream, poll) => do_write_all(stream, poll),
            BlockingSocketStream::Unix(stream, poll) => do_write_all(stream, poll),
        }
    }
}

/// A struct representing a socket address.
#[derive(Debug)]
pub enum SocketAddr {
    /// TCP socket address.
    Tcp(std::net::SocketAddr),
    /// Unix socket address.
    Unix(std::os::unix::net::SocketAddr),
}

/// A structure representing a socket error.
#[derive(Debug)]
pub struct SocketError {
    error: ::std::io::Error,
}

impl SocketError {
    // Creates a new socket error.
    pub fn new(error: ::std::io::Error) -> SocketError {
        SocketError { error }
    }

    /// Gets the kind of the socket error.
    pub fn raw_os_error(&self) -> Option<i32> {
        self.error.raw_os_error()
    }

    /// Gets the kind of the socket error.
    pub fn kind(&self) -> ::std::io::ErrorKind {
        self.error.kind()
    }
}

impl fmt::Display for SocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SocketError: {}", self.error)
    }
}

impl From<SocketError> for io::Error {
    fn from(err: SocketError) -> Self {
        err.error
    }
}

impl From<std::io::Error> for SocketError {
    fn from(error: std::io::Error) -> Self {
        SocketError::new(error)
    }
}

impl Error for SocketError {}
