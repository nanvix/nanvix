// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ReadExact;
use ::log::error;
use ::std::io::Result;
use ::tokio::{
    io::AsyncReadExt,
    net::{
        tcp,
        unix,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Reader half of a connected socket that can only be read from.
///
pub enum SocketStreamReader {
    /// Underlying TCP read half.
    Tcp(tcp::OwnedReadHalf),
    /// Underlying Unix read half.
    Unix(unix::OwnedReadHalf),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketStreamReader {
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
            SocketStreamReader::Tcp(reader) => reader.read(buf).await,
            SocketStreamReader::Unix(reader) => reader.read(buf).await,
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
}

impl ReadExact for SocketStreamReader {
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
            SocketStreamReader::Tcp(stream) => stream.read_exact(buf).await,
            SocketStreamReader::Unix(stream) => stream.read_exact(buf).await,
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
