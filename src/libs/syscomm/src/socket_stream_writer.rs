// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::WriteAll;
use ::log::error;
use ::std::io::{
    IoSlice,
    Result,
};
use ::tokio::io::AsyncWriteExt;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Writer half of a connected socket that can only be written to.
///
pub enum SocketStreamWriter {
    /// Underlying TCP write half.
    Tcp(tokio::net::tcp::OwnedWriteHalf),
    /// Underlying Unix write half.
    Unix(tokio::net::unix::OwnedWriteHalf),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SocketStreamWriter {
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
            SocketStreamWriter::Tcp(stream) => stream.write(buf).await,
            SocketStreamWriter::Unix(stream) => stream.write(buf).await,
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
    /// Writes all bytes from the provided I/O slices using vectored (scatter/gather) I/O.
    /// This avoids extra allocations and copies when coalescing multi-part frames
    /// (e.g., frame-type byte + length prefix + payload) into a single write.
    ///
    /// # Parameters
    ///
    /// - `bufs`: Mutable slice of I/O slices to write. Consumed slices are advanced internally.
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
    pub async fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> Result<()> {
        let mut slices: &mut [IoSlice<'_>] = bufs;
        while !slices.is_empty() {
            let n: usize = match self {
                SocketStreamWriter::Tcp(stream) => stream.write_vectored(slices).await,
                SocketStreamWriter::Unix(stream) => stream.write_vectored(slices).await,
            }
            .map_err(|error| {
                let reason: String = format!("write_all_vectored(): {error}");
                error!("{reason}");
                error
            })?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "write_all_vectored(): write returned 0 bytes",
                ));
            }
            IoSlice::advance_slices(&mut slices, n);
        }
        Ok(())
    }
}

impl WriteAll for SocketStreamWriter {
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
            SocketStreamWriter::Tcp(stream) => stream.write_all(buf).await,
            SocketStreamWriter::Unix(stream) => stream.write_all(buf).await,
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
