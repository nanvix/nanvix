// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::{
        size_t,
        ssize_t,
    },
    unistd::{
        self,
        message::{
            ReadRequest,
            ReadResponse,
        },
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::config::constants::KILOBYTE;
use ::core::cmp;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::not_unsafe_ptr_arg_deref)] // TODO: Wrap this in a safe function.
pub fn read(fd: i32, buffer: *mut u8, count: size_t) -> ssize_t {
    let pid: ProcessIdentifier = match crate::unistd::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Check if buffer is invalid.
    if buffer.is_null() {
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Check if count is invalid.
    if count == 0 {
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Construct buffer from raw parts.
    let buffer: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buffer, count as usize) };

    let mut total_read: ssize_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize = cmp::min(ReadResponse::BUFFER_SIZE, buffer.len() - offset);

        // Build request and send it.
        let request: Message = ReadRequest::build(pid, fd, chunk_size as size_t);
        if let Err(e) = ::nvx::ipc::send(&request) {
            return e.code.into_errno();
        }

        // Receive response.
        let response: Message = match ::nvx::ipc::recv() {
            Ok(response) => response,
            Err(e) => return e.code.into_errno(),
        };

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(e) => return e.into_errno(),
                Err(_) => return ErrorCode::InvalidMessage.into_errno(),
            }
        } else {
            // System call succeeded, parse response.
            match LinuxDaemonMessage::try_from_bytes(response.payload) {
                // Response was successfully parsed.
                Ok(message) => match message.header {
                    // Response was successfully parsed.
                    LinuxDaemonMessageHeader::ReadResponse => {
                        // Parse response.
                        let response: ReadResponse = ReadResponse::from_bytes(message.payload);

                        // Display progress if not STDIN.
                        if fd != unistd::STDIN_FILENO && total_read % KILOBYTE as i32 == 0 {
                            let percentage = (total_read as f64 / buffer.len() as f64) * 100.0;
                            ::nvx::trace!(
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
                        total_read += response.count;
                        offset += chunk_size;

                        // Check for partial read.
                        if (response.count as usize) < chunk_size {
                            break;
                        }
                    },
                    _ => return ErrorCode::InvalidMessage.into_errno(),
                },
                // Response was not successfully parsed.
                Err(_) => return ErrorCode::InvalidMessage.into_errno(),
            }
        }
    }

    total_read
}
