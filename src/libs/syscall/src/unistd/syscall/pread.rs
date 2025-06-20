// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        PartialReadRequest,
        PartialReadResponse,
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
use sysapi::sys_types::{
    off_t,
    c_size_t,
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
/// - `buffer`: Buffer to read.
/// - `offset`: Offset to read from.
///
/// # Returns
///
/// Upon successful completion, `pread()` returns the number of bytes read. Otherwise, it
/// returns an error.
///
pub fn pread(fd: RawFileDescriptor, buffer: &mut [u8], offset: off_t) -> Result<c_size_t, Error> {
    ::syslog::trace!("pread(): fd={}, buffer={:?}, offset={}", fd, buffer, offset);

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    let mut total_read: c_size_t = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let chunk_size: usize =
            cmp::min(PartialReadResponse::BUFFER_SIZE, buffer.len() - buffer_offset);

        // Build request and send it.
        let request: Message = PartialReadRequest::build(
            pid,
            fd,
            chunk_size as c_size_t,
            offset + buffer_offset as off_t,
        );
        ::sys::kcall::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "pread(): failed (fd={}, buffer.len={}, offset={}, error_code={})",
                fd,
                buffer.len(),
                offset,
                { response.status }
            );

            match ErrorCode::try_from(response.status) {
                // System call failed, return error.
                Ok(error_code) => return Err(Error::new(error_code, "pread() failed")),
                // System call failed, return unknown error.
                Err(error) => {
                    ::syslog::error!("pread(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "pread() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::PartialReadResponse => {
                    // Parse response.
                    let response: PartialReadResponse =
                        PartialReadResponse::from_bytes(message.payload);

                    // Check if any data was read.
                    if response.count == 0 {
                        break;
                    }

                    // Copy response buffer to user buffer.
                    buffer[buffer_offset..buffer_offset + chunk_size]
                        .copy_from_slice(&response.buffer[..chunk_size]);
                    total_read += response.count as c_size_t;
                    buffer_offset += chunk_size;

                    // Check for partial read.
                    if (response.count as usize) < chunk_size {
                        break;
                    }
                },
                header => {
                    ::syslog::error!(
                        "pread(): failed to parse response (fd={}, buffer.len={}, offset={}, \
                         header={:?})",
                        fd,
                        buffer.len(),
                        offset,
                        header
                    );
                    return Err(Error::new(ErrorCode::TryAgain, "pread() failed"));
                },
            }
        }
    }

    Ok(total_read)
}
