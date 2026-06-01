// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::message::{
        ReceiveSocketRequest,
        ReceiveSocketResponse,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn recv(sockfd: i32, buffer: &mut [u8], flags: c_int) -> Result<usize, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    let mut total_read: usize = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let recv_len: usize =
            cmp::min(ReceiveSocketResponse::BUFFER_SIZE, buffer.len() - buffer_offset);

        // Build request and send it.
        let request: Message = ReceiveSocketRequest::build(tid, sockfd, recv_len as u32, flags);
        ::sys::kcall::ipc::__kcall_send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            let error_code = ErrorCode::try_from(response.status)?;
            return Err(Error::new(error_code, "failed to receive data on socket"));
        } else {
            // System call succeeded, parse response.
            match SystemCallMessage::try_from_bytes(response.payload) {
                // Response was successfully parsed.
                Ok(message) => match message.header {
                    // Response was successfully parsed.
                    SystemCallMessageHeader::ReceiveSocketResponse => {
                        // Parse response.
                        let response: ReceiveSocketResponse =
                            ReceiveSocketResponse::from_bytes(message.payload);

                        // Check if any data was received.
                        if response.count == 0 {
                            break;
                        }

                        // Copy response buffer to user buffer.
                        buffer[buffer_offset..buffer_offset + response.count as usize]
                            .copy_from_slice(&response.buffer[..response.count as usize]);
                        total_read += response.count as usize;
                        buffer_offset += response.count as usize;

                        // Check for partial receive.
                        if (response.count as usize) < recv_len {
                            break;
                        }
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

    Ok(total_read)
}
