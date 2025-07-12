// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::anyhow::Result;
use ::log::error;
use ::mio::{
    Interest,
    Registry,
    Token,
    net::{
        TcpListener,
        TcpStream,
        UnixListener,
        UnixStream,
    },
};
use ::std::{
    fs,
    io::{
        self,
        ErrorKind,
        Read,
        Write,
    },
    mem,
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
            SocketType::Tcp => Ok(SocketListener::Tcp(TcpListener::bind(addr.parse()?)?)),
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
    pub fn accept(&self) -> io::Result<SocketStream> {
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

impl mio::event::Source for SocketListener {
    fn register(&mut self, registry: &Registry, token: Token, interests: Interest) -> io::Result<()> {
        match self {
            SocketListener::Tcp(listener) => listener.register(registry, token, interests),
            SocketListener::Unix { listener, path: _ } => listener.register(registry, token, interests),
        }
    }

    fn reregister(&mut self, registry: &Registry, token: Token, interests: Interest) -> io::Result<()> {
        match self {
            SocketListener::Tcp(listener) => listener.reregister(registry, token, interests),
            SocketListener::Unix { listener, path: _ } => listener.reregister(registry, token, interests),
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
    pub fn connect(typ: SocketType, addr: String) -> Result<SocketStream> {
        match typ {
            SocketType::Tcp => {
                let stream: TcpStream = TcpStream::connect(addr.parse()?)?;
                Ok(SocketStream::Tcp(stream))
            },
            SocketType::Unix => {
                let stream: UnixStream = UnixStream::connect(addr)?;
                Ok(SocketStream::Unix(stream))
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

    /// Read one message from the gateway comprising a header with a size
    /// (u32 LE) and then the body with as many bytes as indicated in the header.
    pub fn read_message_from_gateway(&mut self) -> Result<Vec<u8>, ::std::io::Error> {
        let mut buffer = Vec::new();
        let mut tmp = [0u8; config::syscomm::GW_READ_BUFFER_LEN];
        let u32_size = mem::size_of::<u32>();
        let mut message_len: Option<usize> = None;

        // Read available data.
        loop {
            match self.read(&mut tmp) {
                Ok(0) => {
                    // Connection closed.
                    if buffer.is_empty() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Connection closed",
                        ));
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Partial message",
                        ));
                    }
                },
                Ok(n) => {
                    buffer.extend_from_slice(&tmp[..n]);

                    // Check if we have enough to parse the length.
                    if message_len.is_none() && buffer.len() >= u32_size {
                        if buffer.len() > u32_size + config::syscomm::MAX_GW_MESSAGE_LEN {
                            // Guard against the buffer length growing too much.
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Message too large",
                            ));
                        }

                        let len =
                            u32::from_le_bytes(buffer[..u32_size].try_into().unwrap()) as usize;
                        message_len = Some(len);
                    }

                    // Check if we are done reading.
                    if let Some(msg_len) = message_len {
                        if buffer.len() >= u32_size + msg_len {
                            // Full message received.
                            let message = buffer[u32_size..u32_size + msg_len].to_vec();
                            return Ok(message);
                        }
                    }

                    // Otherwise, continue reading.
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Non-blocking mode: no data yet, return error or retry.
                    return Err(e);
                },
                Err(e) => return Err(e),
            }
        }
    }

    /// Send a message to the gateway by prepending the buffer size (u32 LE)
    /// to the buffer body.
    pub fn send_message_to_gateway(&mut self, buffer: &[u8]) -> Result<(), SocketError> {
        let mut out_buffer = Vec::with_capacity(mem::size_of::<u32>() + buffer.len());
        if buffer.len() > u32::MAX as usize {
            return Err(SocketError::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Buffer size exceeds u32::MAX",
            )));
        }
        let buffer_len: u32 = buffer
            .len()
            .try_into()
            .expect("Buffer size already validated");

        out_buffer.extend_from_slice(&buffer_len.to_le_bytes());
        out_buffer.extend_from_slice(buffer);

        self.write_all(&out_buffer)?;
        Ok(())
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

impl mio::event::Source for SocketStream {
    fn register(&mut self, registry: &Registry, token: Token, interests: Interest) -> io::Result<()> {
        match self {
            SocketStream::Tcp(stream) => stream.register(registry, token, interests),
            SocketStream::Unix(stream) => stream.register(registry, token, interests),
        }
    }

    fn reregister(&mut self, registry: &Registry, token: Token, interests: Interest) -> io::Result<()> {
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
    pub fn kind(&self) -> ::std::io::ErrorKind {
        self.error.kind()
    }
}
