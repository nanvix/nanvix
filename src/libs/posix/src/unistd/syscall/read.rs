// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::types::size_t,
    unistd::{
        self,
        message::{
            ReadRequest,
            ReadResponse,
        },
        STDIN_FILENO,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::config::constants::KILOBYTE;
use ::core::cmp;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to read into.
///
/// # Returns
///
/// Upon successful completion, `read()` returns the number of bytes read. Otherwise, it returns an
/// error.
///
pub fn read(fd: RawFileDescriptor, buffer: &mut [u8]) -> Result<size_t, Error> {
    // Skip logging for stdin to avoid spamming the output.
    if fd != STDIN_FILENO {
        ::syslog::trace!("read(): fd={:?}, buffer.len={:?}", fd, buffer.len());
    }

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    let mut total_read: size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize = cmp::min(ReadResponse::BUFFER_SIZE, buffer.len() - offset);

        // Build request and send it.
        let request: Message = ReadRequest::build(pid, fd, chunk_size as size_t);
        ::nvx::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::nvx::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "read(): failed (fd={:?}, buffer.len={:?}, error_code={:?})",
                fd,
                buffer.len(),
                { response.status }
            );

            match ErrorCode::try_from(response.status) {
                // Error code was successfully parsed.
                Ok(error_code) => return Err(Error::new(error_code, "read() failed")),
                // Error code was not successfully parsed.
                Err(error) => {
                    ::syslog::error!(
                        "read(): failed (fd={:?}, buffer.len={:?}, error_code={:?})",
                        fd,
                        buffer.len(),
                        error
                    );
                    return Err(Error::new(ErrorCode::TryAgain, "read() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::ReadResponse => {
                    // Parse response.
                    let response: ReadResponse = ReadResponse::from_bytes(message.payload);

                    // Display progress if not STDIN.
                    if fd != unistd::STDIN_FILENO && total_read % KILOBYTE as size_t == 0 {
                        let percentage = (total_read as f64 / buffer.len() as f64) * 100.0;
                        ::syslog::trace!(
                            "read(): {:?}/{:?} bytes read from fd={} ({:.2}%)",
                            total_read,
                            buffer.len(),
                            fd,
                            percentage
                        );
                    }

                    // Check if any data was read.
                    if response.count == 0 {
                        break;
                    }

                    // Copy response buffer to user buffer.
                    buffer[offset..offset + chunk_size]
                        .copy_from_slice(&response.buffer[..chunk_size]);
                    total_read += response.count as size_t;
                    offset += chunk_size;

                    // Check for partial read.
                    if (response.count as usize) < chunk_size {
                        break;
                    }
                },
                header => {
                    ::syslog::error!(
                        "read(): failed to parse response (fd={:?}, buffer.len={:?}, header={:?})",
                        fd,
                        buffer.len(),
                        header
                    );
                    return Err(Error::new(ErrorCode::InvalidMessage, "read() failed"));
                },
            }
        }
    }

    Ok(total_read)
}
