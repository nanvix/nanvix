// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::NanvixUserland;
use ::litebox::platform::{
    StdioOutStream,
    StdioProvider,
    StdioReadError,
    StdioStream,
    StdioWriteError,
};
use posix::unistd::{
    self,
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl StdioProvider for NanvixUserland {
    ///
    /// # Description
    ///
    /// Reads data from the standard input.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer to read data into.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes read is returned. Upon failure, an error
    /// is returned instead.
    ///
    /// # Safety
    ///
    /// This function panics if the read operation fails.
    ///
    fn read_from_stdin(&self, buf: &mut [u8]) -> Result<usize, StdioReadError> {
        // Read from standard input and check for errors.
        match unistd::read(STDIN_FILENO, buf) {
            Ok(nread) => Ok(nread as usize),
            Err(error) => panic!("read_from_stdin(): {:?}", error),
        }
    }

    ///
    /// # Description
    ///
    /// Writes data to the standard output.
    ///
    /// # Parameters
    ///
    /// - `stream`: The stream to write to.
    /// - `buf`: The buffer containing the data to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes written is returned. Upon failure, an error
    /// is returned instead.
    ///
    /// # Safety
    ///
    /// This function panics if the write operation fails.
    ///
    fn write_to(&self, _stream: StdioOutStream, buf: &[u8]) -> Result<usize, StdioWriteError> {
        // Write to standard output and check for errors.
        match unistd::write(STDOUT_FILENO, buf) {
            Ok(n) => Ok(n as usize),
            Err(error) => panic!("write_to(): {:?}", error),
        }
    }

    ///
    /// # Description
    ///
    /// Checks if the given stream is a TTY.
    ///
    /// # Parameters
    ///
    /// - `stream`: The stream to check.
    ///
    /// # Returns
    ///
    /// True if the stream is a TTY, false otherwise.
    ///
    fn is_a_tty(&self, stream: StdioStream) -> bool {
        // Check if the given stream is a TTY.
        match stream {
            StdioStream::Stdout => unistd::isatty(STDOUT_FILENO).unwrap_or(false),
            StdioStream::Stderr => unistd::isatty(STDERR_FILENO).unwrap_or(false),
            StdioStream::Stdin => unistd::isatty(STDIN_FILENO).unwrap_or(false),
        }
    }
}
