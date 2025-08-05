// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::anyhow::Result;
use ::log::error;
use ::std::{
    error::Error,
    fs,
    fmt,
    io::{
        self,
        ErrorKind,
        Read,
        Write,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    os::unix::net::{
        UnixListener,
        UnixStream,
    },
};

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
            SocketType::Tcp => Ok(SocketListener::Tcp(TcpListener::bind(addr)?)),
            SocketType::Unix => Ok(SocketListener::Unix { listener: UnixListener::bind(&addr)?, path: addr.clone() }),
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
    pub fn accept(&self) -> Result<SocketStream> {
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
}

impl Drop for SocketListener {
    fn drop(&mut self) {
        match self {
            SocketListener::Tcp(_) => {},
            SocketListener::Unix { listener: _, path } => {
                match fs::remove_file(path.clone()) {
                    Ok(_) => {},
                    Err(ref e) if e.kind() == ErrorKind::NotFound => {},
                    Err(e) => error!("error removing UNIX socket (path={path}, error={e:?})"),
                }
            }
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
    pub fn connect(typ: SocketType, addr: String) -> Result<SocketStream> {
        match typ {
            SocketType::Tcp => {
                let stream: TcpStream = TcpStream::connect(addr)?;
                Ok(SocketStream::Tcp(stream))
            },
            SocketType::Unix => {
                let stream: UnixStream = UnixStream::connect(addr)?;
                Ok(SocketStream::Unix(stream))
            },
        }
    }

    pub fn try_clone(&self) -> Result<SocketStream, ::std::io::Error> {
        match self {
            SocketStream::Tcp(stream) => {
                let stream: TcpStream = stream.try_clone()?;
                Ok(SocketStream::Tcp(stream))
            }
            SocketStream::Unix(stream) => {
                let stream: UnixStream = stream.try_clone()?;
                Ok(SocketStream::Unix(stream))
            }
        }
    }

    /// Sets a socket stream to non-blocking mode.
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), ::std::io::Error> {
        match self {
            SocketStream::Tcp(stream) => stream.set_nonblocking(nonblocking),
            SocketStream::Unix(stream) => stream.set_nonblocking(nonblocking),
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

    /// Reads data from a socket stream.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ::std::io::Error> {
        let result = match self {
            SocketStream::Tcp(stream) => stream.read_exact(buf),
            SocketStream::Unix(stream) => stream.read_exact(buf),
        };

        match result {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
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

/// Implement Read trait for SocketStream.
impl Read for SocketStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            SocketStream::Tcp(stream) => stream.read(buf),
            SocketStream::Unix(stream) => stream.read(buf),
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

impl Error for SocketError {}
