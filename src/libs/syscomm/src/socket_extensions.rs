// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    future::Future,
    io::Result,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Trait for writing all bytes from a buffer into a socket stream.
///
pub trait WriteAll {
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
    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = Result<()>> + Send;
}

///
/// # Description
///
/// Trait for reading exactly the number of bytes required to fill a buffer from a socket stream.
///
pub trait ReadExact {
    ///
    /// # Description
    ///
    /// Reads exactly the number of bytes required to fill the provided buffer from the socket
    /// stream.
    ///
    /// # Parameters
    ///
    /// - `buf`: Buffer to fill with the read bytes.
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
    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize>> + Send;
}
