// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ReadExact,
    SocketAddr,
    SocketStreamReader,
    SocketStreamWriter,
    WriteAll,
};
use ::log::{
    error,
    trace,
};
use ::std::io::Result;
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::{
        tcp,
        unix,
        TcpStream,
        UnixStream,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// A connected socket.
#[derive(Debug)]
pub enum SocketStream {
    /// Underlying TCP socket stream.
    Tcp(TcpStream),
    /// Underlying Unix socket stream.
    Unix(UnixStream),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketStream {
    ///
    /// # Description
    ///
    /// Splits a socket stream into a read half and a write half.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the read half and the write half of the socket
    /// stream.
    ///
    pub fn split(self) -> (SocketStreamReader, SocketStreamWriter) {
        trace!("split(): self={self:?}");
        // Match socket type.
        match self {
            // Split a TCP socket stream.
            SocketStream::Tcp(stream) => {
                let (reader, writer): (tcp::OwnedReadHalf, tcp::OwnedWriteHalf) =
                    stream.into_split();
                (SocketStreamReader::Tcp(reader), SocketStreamWriter::Tcp(writer))
            },
            // Split a Unix socket stream.
            SocketStream::Unix(stream) => {
                let (reader, writer): (unix::OwnedReadHalf, unix::OwnedWriteHalf) =
                    stream.into_split();
                (SocketStreamReader::Unix(reader), SocketStreamWriter::Unix(writer))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Reads bytes from a socket stream into a buffer.
    ///
    /// # Parameters
    ///
    /// - `buf`: Mutable buffer to store the read bytes.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: The number of bytes read into the buffer.
    /// - On Failure: An error value.
    ///
    /// # Cancellation
    ///
    /// This function is cancel-safe. If the future is cancelled before completion, no bytes
    /// are read from the socket stream, and the buffer remains unchanged.
    ///
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // No tracing on data path.

        // Read bytes.
        let result: Result<usize> = match self {
            SocketStream::Tcp(stream) => stream.read(buf).await,
            SocketStream::Unix(stream) => stream.read(buf).await,
        };

        // Parse result.
        match result {
            Ok(n) => Ok(n),
            Err(error) => {
                let reason: String = format!("read(): {error}");
                error!("read(): {reason}");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Writes bytes from a buffer into a socket stream.
    ///
    /// # Parameters
    ///
    /// - `buf`: Buffer containing the bytes to write.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: The number of bytes written from the buffer.
    /// - On Failure: An error value.
    ///
    /// # Cancellation
    ///
    /// This function is cancel-safe. If the future is cancelled before completion, no
    /// additional bytes are written to the socket stream, and the provided buffer remains
    /// unchanged.
    ///
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        // No tracing on data path.

        // Write bytes.
        let result: Result<usize> = match self {
            SocketStream::Tcp(stream) => stream.write(buf).await,
            SocketStream::Unix(stream) => stream.write(buf).await,
        };

        // Parse result.
        match result {
            Ok(n) => Ok(n),
            Err(error) => {
                let reason: String = format!("write(): {error}");
                error!("write(): {reason}");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Shuts down the write half of the socket stream so that peers observe an EOF.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` when the shutdown succeeds; returns an error if the underlying
    /// transport refuses the request.
    ///
    pub async fn shutdown_write(&mut self) -> Result<()> {
        let result: Result<()> = match self {
            SocketStream::Tcp(stream) => stream.shutdown().await,
            SocketStream::Unix(stream) => stream.shutdown().await,
        };

        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let reason: String = format!("shutdown_write(): {error}");
                error!("shutdown_write(): {reason}");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Gets the peer address of a socket stream.
    ///
    /// # Returns
    ///
    /// On success, this function returns the peer address of the socket stream. On failure, it
    /// returns an error value.
    ///
    pub fn peer_addr(&self) -> Result<SocketAddr> {
        match self {
            SocketStream::Tcp(stream) => match stream.peer_addr() {
                Ok(addr) => Ok(SocketAddr::Tcp(addr)),
                Err(error) => {
                    let reason: String = format!("peer_addr(): {error}");
                    error!("peer_addr(): {reason}");
                    Err(error)
                },
            },
            SocketStream::Unix(stream) => match stream.peer_addr() {
                Ok(addr) => Ok(SocketAddr::Unix(addr)),
                Err(error) => {
                    let reason: String = format!("peer_addr(): {error}");
                    error!("peer_addr(): {reason}");
                    Err(error)
                },
            },
        }
    }
}

impl ReadExact for SocketStream {
    ///
    /// # Description
    ///
    /// Reads exactly the number of bytes required to fill the provided buffer.
    ///
    /// # Parameters
    ///
    /// - `buf`: Mutable buffer that must be fully populated with data from the socket stream.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: The number of bytes read into the buffer.
    /// - On Failure: An error value.
    ///
    /// # Cancellation
    ///
    /// This function is not cancel-safe. If the future is cancelled before completion, the number
    /// of bytes that were read prior to cancellation is unspecified.
    ///
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<usize> {
        // No tracing on data path.

        // Read bytes.
        let result: Result<usize> = match self {
            SocketStream::Tcp(stream) => stream.read_exact(buf).await,
            SocketStream::Unix(stream) => stream.read_exact(buf).await,
        };

        // Parse result.
        match result {
            Ok(n) => Ok(n),
            Err(error) => {
                let reason: String = format!("read_exact(): {error}");
                error!("read_exact(): {reason}");
                Err(error)
            },
        }
    }
}

impl WriteAll for SocketStream {
    ///
    /// # Description
    ///
    /// Writes all bytes from the provided buffer into the socket stream.
    ///
    /// # Parameters
    ///
    /// - `buf`: Buffer containing the bytes to write.
    ///
    /// # Returns
    ///
    /// This function returns a future that, when resolved, yields either:
    /// - On Success: An empty tuple after all bytes have been written.
    /// - On Failure: An error value.
    ///
    /// # Cancellation
    ///
    /// This function is not cancel-safe. If the future is cancelled before completion, the number
    /// of bytes that were written prior to cancellation is unspecified.
    ///
    async fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        // No tracing on data path.

        // Write bytes.
        let result: Result<()> = match self {
            SocketStream::Tcp(stream) => stream.write_all(buf).await,
            SocketStream::Unix(stream) => stream.write_all(buf).await,
        };

        // Parse result.
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let reason: String = format!("write_all(): {error}");
                error!("write_all(): {reason}");
                Err(error)
            },
        }
    }
}
