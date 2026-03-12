// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::SocketStream;
use ::log::{
    error,
    trace,
};
use ::std::{
    fs,
    io::{
        Error,
        Result,
    },
    net::SocketAddr,
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

/// A socket that is bound and can accept incoming connections.
#[derive(Debug)]
pub enum SocketListener {
    /// TCP socket.
    Tcp {
        /// TCP listener.
        listener: TcpListener,
        /// Socket address.
        addr: SocketAddr,
    },
    /// Unix socket.
    Unix {
        /// Unix listener.
        listener: UnixListener,
        /// Socket path.
        path: String,
    },
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketListener {
    ///
    /// # Description
    ///
    /// Accepts an incoming connection.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: A socket stream representing the accepted connection.
    /// - On Failure: An error value.
    ///
    pub async fn accept(&self) -> Result<SocketStream> {
        // Match socket type.
        match self {
            // TCP socket.
            SocketListener::Tcp {
                listener,
                addr: _addr,
            } => {
                let (stream, sockaddr): (TcpStream, std::net::SocketAddr) =
                    match listener.accept().await {
                        Ok(res) => res,
                        Err(error) => {
                            let reason: String = format!("accept(): {error}");
                            error!("accept(): {reason}");
                            return Err(Error::new(error.kind(), reason));
                        },
                    };

                // Disable Nagle's algorithm so small IKC frames are sent immediately
                // instead of being delayed up to 40 ms by the TCP delayed-ACK interaction.
                stream.set_nodelay(true).map_err(|error| {
                    Error::new(
                        error.kind(),
                        format!("accept(): set_nodelay failed: {error} (peer={sockaddr})"),
                    )
                })?;
                Ok(SocketStream::Tcp(stream))
            },
            // Unix socket.
            SocketListener::Unix {
                listener,
                path: _path,
            } => {
                let (stream, _sockaddr): (UnixStream, ::tokio::net::unix::SocketAddr) =
                    match listener.accept().await {
                        Ok(res) => res,
                        Err(error) => {
                            let reason: String = format!("accept(): {error}");
                            error!("accept(): {reason}");
                            return Err(Error::new(error.kind(), reason));
                        },
                    };
                Ok(SocketStream::Unix(stream))
            },
        }
    }
}

impl Drop for SocketListener {
    fn drop(&mut self) {
        trace!("drop(): {:?}", self);
        match self {
            SocketListener::Tcp {
                listener: _,
                addr: _,
            } => {},
            SocketListener::Unix { listener: _, path } => {
                // Remove underlying socket file, because dropping the listener just closes the file
                // descriptor.
                if let Err(error) = fs::remove_file(path.clone()) {
                    error!("drop(): {error}")
                }
            },
        }
    }
}
