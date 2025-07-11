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
};

//==================================================================================================
// Constants
//==================================================================================================

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
                Ok(SocketStream::Tcp(stream, None))
            },
            SocketListener::Unix { listener, path: _ } => {
                let (stream, _sockaddr): (UnixStream, std::os::unix::net::SocketAddr) =
                    listener.accept()?;
                Ok(SocketStream::Unix(stream, None))
            },
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
    Tcp(TcpStream, Option<Poll>),
    /// Unix socket stream.
    Unix(UnixStream, Option<Poll>),
}

impl SocketStream {
    /// Creates a new socket stream.
    pub fn connect(typ: SocketType, addr: String) -> Result<SocketStream> {
        match typ {
            SocketType::Tcp => {
                let stream: TcpStream = TcpStream::connect(addr.parse()?)?;
                Ok(SocketStream::Tcp(stream, None))
            },
            SocketType::Unix => {
                let stream: UnixStream = UnixStream::connect(addr)?;
                Ok(SocketStream::Unix(stream, None))
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
    /// This approach is intended to be used when we are only monitoring one stream, potentially
    /// across different worker threads. To monitor multiple streams, you should use a global poll.
    ///
    pub fn set_blocking(&mut self) -> io::Result<()> {
        match self {
            SocketStream::Tcp(stream, ref mut poll) => {
                // Initialize Poll structure if it's not already set
                if poll.is_none() {
                    let poll_instance = Poll::new()
                        .map_err(|_| io::Error::other("failed to create Poll instance"))?;

                    poll_instance
                        .registry()
                        .register(
                            stream,
                            BLOCKING_THREAD_TOKEN,
                            Interest::READABLE | Interest::WRITABLE,
                        )
                        .map_err(|_| io::Error::other("failed to register thread to poll"))?;

                    *poll = Some(poll_instance);
                }

                Ok(())
            },
            SocketStream::Unix(stream, ref mut poll) => {
                if poll.is_none() {
                    let poll_instance = Poll::new()
                        .map_err(|_| io::Error::other("failed to create Poll instance"))?;

                    poll_instance
                        .registry()
                        .register(
                            stream,
                            BLOCKING_THREAD_TOKEN,
                            Interest::READABLE | Interest::WRITABLE,
                        )
                        .map_err(|_| io::Error::other("failed to register thread to poll"))?;

                    *poll = Some(poll_instance);
                }

                Ok(())
            },
        }
    }

    /// Writes data to a socket stream.
    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), SocketError> {
        let result = match self {
            SocketStream::Tcp(stream, _) => stream.write_all(buf),
            SocketStream::Unix(stream, _) => stream.write_all(buf),
        };

        match result {
            Ok(_) => Ok(()),
            Err(error) => Err(SocketError { error }),
        }
    }

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
    pub fn try_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            SocketStream::Tcp(stream, _) => stream.read(buf),
            SocketStream::Unix(stream, _) => stream.read(buf),
        }
    }

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
            SocketStream::Tcp(stream, poll) => {
                if let Some(poll_instance) = poll {
                    let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

                    poll_instance.poll(&mut events, None)?;
                    stream.read(buf)
                } else {
                    let reason = "tried to perform blocking read on a non-blocking thread";
                    error!("{reason}");
                    Err(io::Error::new(io::ErrorKind::InvalidData, reason))
                }
            },
            SocketStream::Unix(stream, poll) => {
                if let Some(poll_instance) = poll {
                    let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

                    poll_instance.poll(&mut events, None)?;
                    stream.read(buf)
                } else {
                    let reason = "tried to perform blocking read on a non-blocking thread";
                    error!("{reason}");
                    Err(io::Error::new(io::ErrorKind::InvalidData, reason))
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Non-blocking read exact implementation.
    ///
    /// # Parameters
    ///
    /// A mutable buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read into the buffer.
    ///
    pub fn try_read_exact(&mut self, buf: &mut [u8]) -> Result<(), SocketError> {
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

        // You may return partial reads if needed; or retry later
        if total_read == buf.len() {
            Ok(())
        } else {
            Err(io::Error::new(ErrorKind::WouldBlock, "need more data").into())
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
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Gets the peer address of a socket stream.
    pub fn peer_addr(&self) -> Result<SocketAddr, SocketError> {
        match self {
            SocketStream::Tcp(stream, _) => match stream.peer_addr() {
                Ok(addr) => Ok(SocketAddr::Tcp(addr)),
                Err(error) => Err(SocketError { error }),
            },
            SocketStream::Unix(stream, _) => match stream.peer_addr() {
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
            SocketStream::Tcp(stream, poll) => {
                if poll.is_some() {
                    let reason = "trying to register a blocking thread to a poll";
                    error!("{reason}");
                    return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
                }

                stream.register(registry, token, interests)
            },
            SocketStream::Unix(stream, poll) => {
                if poll.is_some() {
                    let reason = "trying to register a blocking thread to a poll";
                    error!("{reason}");
                    return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
                }

                stream.register(registry, token, interests)
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
            SocketStream::Tcp(stream, poll) => {
                if poll.is_some() {
                    let reason = "trying to register a blocking thread to a poll";
                    error!("{reason}");
                    return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
                }

                stream.reregister(registry, token, interests)
            },
            SocketStream::Unix(stream, poll) => {
                if poll.is_some() {
                    let reason = "trying to register a blocking thread to a poll";
                    error!("{reason}");
                    return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
                }

                stream.reregister(registry, token, interests)
            },
        }
    }

    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        match self {
            SocketStream::Tcp(stream, _) => stream.deregister(registry),
            SocketStream::Unix(stream, _) => stream.deregister(registry),
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
