// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::message::{
        SendSocketRequest,
        SendSocketResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::cmp;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn send(sockfd: c_int, buffer: &[u8], flags: c_int) -> Result<usize, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd = crate::fdtable::resolve_socket(sockfd, "send")?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    let mut total_written: usize = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize = cmp::min(SendSocketRequest::BUFFER_SIZE, buffer.len() - offset);
        let mut chunk: [u8; SendSocketRequest::BUFFER_SIZE] = [0; SendSocketRequest::BUFFER_SIZE];
        chunk[..chunk_size].copy_from_slice(&buffer[offset..offset + chunk_size]);

        // Build request and send it.
        let request: Message =
            SendSocketRequest::build(tid, sockfd, chunk_size as c_size_t, flags, chunk);
        ::sys::kcall::ipc::__kcall_send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            let error_code = ErrorCode::try_from(response.status)?;
            return Err(Error::new(error_code, "failed to send data through socket"));
        } else {
            // System call succeeded, parse response.
            match SystemCallMessage::try_from_bytes(response.payload) {
                // Response was successfully parsed.
                Ok(message) => match message.header {
                    // Response was successfully parsed.
                    SystemCallMessageHeader::SendSocketResponse => {
                        // Parse response.
                        let response: SendSocketResponse =
                            SendSocketResponse::from_bytes(message.payload);

                        // Update total written count.
                        total_written += response.count as usize;
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
