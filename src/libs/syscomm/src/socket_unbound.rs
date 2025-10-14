// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SocketListener,
    SocketStream,
    SocketType,
};
use ::std::{
    io::{
        self,
        Error,
        ErrorKind,
        Result,
    },
    net::SocketAddr,
};
use ::syslog::{
    error,
    trace,
};
use ::tokio::net::{
    TcpListener,
    TcpStream,
    UnixListener,
    UnixStream,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Unbound socket.
#[derive(Debug, Clone)]
pub struct UnboundSocket {
    /// Socket type.
    typ: SocketType,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UnboundSocket {
    ///
    /// # Description
    ///
    /// Creates a new unbound socket.
    ///
    /// # Parameters
    ///
    /// - `typ`: Socket type.
    ///
    /// # Returns
    ///
    /// This function returns a new unbound socket of the specified type.
    ///
    pub fn new(typ: SocketType) -> UnboundSocket {
        UnboundSocket { typ }
    }

    ///
    /// # Description
    ///
    /// Connects to a remote socket.
    ///
    /// # Parameters
    ///
    /// - `addr`: Socket address.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: A socket stream connected to the specified address.
    /// - On Failure: An error value.
    ///
    pub async fn connect(self, addr: String) -> Result<SocketStream> {
        trace!("connect(): addr={addr}");
        // Match socket type.
        match self.typ {
            // Connect to a TCP socket.
            SocketType::Tcp => {
                // Parse socket address.
                let addr: SocketAddr = match addr.parse::<SocketAddr>() {
                    Ok(addr) => addr,
                    Err(error) => {
                        return Err(io::Error::new(
                            ErrorKind::InvalidInput,
                            format!("{error} (addr={addr})"),
                        ));
                    },
                };
                let stream: TcpStream = match TcpStream::connect(addr).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        return Err(io::Error::new(error.kind(), format!("{error} (addr={addr})")));
                    },
                };
                Ok(SocketStream::Tcp(stream))
            },
            // Connect to a unix domain socket.
            SocketType::Unix => {
                let stream: UnixStream = match UnixStream::connect(addr).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        return Err(Error::new(error.kind(), format!("{error}")));
                    },
                };
                Ok(SocketStream::Unix(stream))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Binds a socket.
    ///
    /// # Parameters
    ///
    /// - `addr`: Socket address.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: A socket that is bound and ready to accept incoming connections.
    /// - On Failure: An error value.
    ///
    pub async fn bind(self, addr: String) -> Result<SocketListener> {
        trace!("bind(): addr={addr}");
        // Match socket type.
        match self.typ {
            // Bind a TCP socket.
            SocketType::Tcp => {
                // Parse socket address.
                let addr: SocketAddr = match addr.parse::<SocketAddr>() {
                    Ok(addr) => addr,
                    Err(error) => {
                        let reason: String = format!("bind(): {error} (addr={addr})");
                        error!("bind(): {reason}");
                        return Err(io::Error::new(ErrorKind::InvalidInput, reason));
                    },
                };
                // Bind socket.
                match TcpListener::bind(addr).await {
                    Ok(listener) => Ok(SocketListener::Tcp { listener, addr }),
                    Err(error) => {
                        let reason: String = format!("bind(): {error} (addr={addr})");
                        error!("bind(): {reason}");
                        Err(io::Error::new(error.kind(), reason))
                    },
                }
            },
            // Bind a unix domain socket.
            SocketType::Unix => match UnixListener::bind(&addr) {
                Ok(listener) => Ok(SocketListener::Unix {
                    listener,
                    path: addr,
                }),
                Err(error) => {
                    let reason: String = format!("bind(): {error}");
                    error!("bind(): {reason}");
                    Err(Error::new(error.kind(), reason))
                },
            },
        }
    }
}
