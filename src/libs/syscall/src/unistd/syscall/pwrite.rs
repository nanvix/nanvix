// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::types::{
        off_t,
        size_t,
    },
    unistd::message::{
        PartialWriteRequest,
        PartialWriteResponse,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::cmp;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ProcessIdentifier,
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
/// - `offset`: Offset to write to.
///
/// # Returns
///
/// Upon successful completion, `pwrite()` returns the number of bytes written. Otherwise, it
/// returns an error.
///
pub fn pwrite(fd: RawFileDescriptor, buffer: &[u8], offset: off_t) -> Result<size_t, Error> {
    ::syslog::trace!("pwrite(): fd={}, buffer={:?}, offset={}", fd, buffer, offset);

    let mut total_written: size_t = 0;
    let mut buffer_offset: usize = 0;

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    while buffer_offset < buffer.len() {
        let chunk_size: usize =
            cmp::min(PartialWriteRequest::BUFFER_SIZE, buffer.len() - buffer_offset);
        let mut chunk: [u8; PartialWriteRequest::BUFFER_SIZE] =
            [0; PartialWriteRequest::BUFFER_SIZE];
        chunk[..chunk_size].copy_from_slice(&buffer[buffer_offset..buffer_offset + chunk_size]);

        // Build request and send it.
        let request: Message = PartialWriteRequest::build(
            pid,
            fd,
            chunk_size as size_t,
            offset + buffer_offset as off_t,
            chunk,
        );
        ::sys::kcall::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether the system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "pwrite(): failed (fd={}, buffer.len={}, error_code={})",
                fd,
                buffer.len(),
                { response.status }
            );

            match ErrorCode::try_from(response.status) {
                // Error code was successfully parsed.
                Ok(error_code) => return Err(Error::new(error_code, "pwritev() failed")),
                // Error code was not parsed.
                Err(error) => {
                    ::syslog::error!("pwrite(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "pwritev() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::PartialWriteResponse => {
                    // Parse response.
                    let message: PartialWriteResponse =
                        PartialWriteResponse::from_bytes(message.payload);

                    // Update total written count.
                    total_written += message.count as size_t;
                    buffer_offset += message.count as usize;
                },
                // Response was not expected.
                header => {
                    ::syslog::error!(
                        "pwrite(): failed to parse response (fd={}, buffer.len={}, header={:?})",
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
