// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::types::size_t,
    unistd::{
        message::{
            WriteRequest,
            WriteResponse,
        },
        STDERR_FILENO,
        STDOUT_FILENO,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
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
/// Writes data to a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to write.
///
/// # Returns
///
/// Upon successful completion, the `write()` system call returns the number of bytes written.
/// Otherwise, it returns an error.
///
pub fn write(fd: RawFileDescriptor, buffer: &[u8]) -> Result<size_t, Error> {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::syslog::trace!("write(): fd={:?}, buffer.len={:?}", fd, buffer.len());
    }

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    let mut total_written: size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize = cmp::min(WriteRequest::BUFFER_SIZE, buffer.len() - offset);
        let mut chunk: [u8; WriteRequest::BUFFER_SIZE] = [0; WriteRequest::BUFFER_SIZE];
        chunk[..chunk_size].copy_from_slice(&buffer[offset..offset + chunk_size]);

        // Build request and send it.
        let request: Message = WriteRequest::build(pid, fd, chunk_size as size_t, chunk);
        ::nvx::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::nvx::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "write(): failed (fd={:?}, buffer.len={:?}, error_code={:?})",
                fd,
                buffer.len(),
                { response.status }
            );

            match ErrorCode::try_from(response.status) {
                // Succeeded to parse error code.
                Ok(error_code) => return Err(Error::new(error_code, "write() failed")),
                // Failed to parse error code, return generic error.
                Err(error) => {
                    ::syslog::error!("write(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "write() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::WriteResponse => {
                    // Parse response.
                    let response: WriteResponse = WriteResponse::from_bytes(message.payload);

                    // Update total written count.
                    total_written += response.count as size_t;
                    offset += chunk_size;
                },
                header => {
                    ::syslog::error!(
                        "write(): failed to parse response (fd={:?}, buffer.len={:?}, header={:?})",
                        fd,
                        buffer.len(),
                        header
                    );
                    return Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"));
                },
            }
        }
    }

    Ok(total_written)
}
