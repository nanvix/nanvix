// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::{
        socket::message::{
            SendSocketRequest,
            SendSocketResponse,
        },
        types::{
            size_t,
            ssize_t,
        },
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

pub fn send(sockfd: c_int, buffer: &[u8], flags: c_int) -> Result<ssize_t, Error> {
    let pid: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    let mut total_written: ssize_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize = cmp::min(SendSocketRequest::BUFFER_SIZE, buffer.len() - offset);
        let mut chunk: [u8; SendSocketRequest::BUFFER_SIZE] = [0; SendSocketRequest::BUFFER_SIZE];
        chunk[..chunk_size].copy_from_slice(&buffer[offset..offset + chunk_size]);

        // Build request and send it.
        let request: Message =
            SendSocketRequest::build(pid, sockfd, chunk_size as size_t, flags, chunk);
        ::sys::kcall::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => {
                    return Err(Error::new(error_code, "failed to send data through socket"))
                },
                Err(e) => return Err(e),
            }
        } else {
            // System call succeeded, parse response.
            match LinuxDaemonMessage::try_from_bytes(response.payload) {
                // Response was successfully parsed.
                Ok(message) => match message.header {
                    // Response was successfully parsed.
                    LinuxDaemonMessageHeader::SendSocketResponse => {
                        // Parse response.
                        let response: SendSocketResponse =
                            SendSocketResponse::from_bytes(message.payload);

                        // Update total written count.
                        total_written += response.count;
                        offset += chunk_size;
                    },
                    _ => {
                        return Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "unexpected message header",
                        ))
                    },
                },
                // Response was not successfully parsed.
                Err(_) => return Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
            }
        }
    }

    Ok(total_written)
}
