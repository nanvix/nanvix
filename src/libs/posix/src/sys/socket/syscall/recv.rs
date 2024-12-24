// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::{
        socket::message::{
            ReceiveSocketRequest,
            ReceiveSocketResponse,
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
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::not_unsafe_ptr_arg_deref)] // TODO: Wrap this in a safe function.
pub fn recv(sockfd: i32, buffer: *mut u8, length: size_t, flags: i32) -> ssize_t {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Check if buffer is invalid.
    if buffer.is_null() {
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Check if count is invalid.
    if length == 0 {
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Construct buffer from raw parts.
    let buffer: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buffer, length as usize) };

    let mut total_read: size_t = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let recv_len: usize =
            cmp::min(ReceiveSocketResponse::BUFFER_SIZE, buffer.len() - buffer_offset);

        // Build request and send it.
        let request: Message = ReceiveSocketRequest::build(pid, sockfd, recv_len as u32, flags);
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
                    LinuxDaemonMessageHeader::ReceiveSocketResponse => {
                        // Parse response.
                        let response: ReceiveSocketResponse =
                            ReceiveSocketResponse::from_bytes(message.payload);

                        // Copy response buffer to user buffer.
                        buffer[buffer_offset..buffer_offset + response.count as usize]
                            .copy_from_slice(&response.buffer[..response.count as usize]);
                        total_read += response.count;
                        buffer_offset += response.count as usize;
                    },
                    _ => return ErrorCode::InvalidMessage.into_errno(),
                },
                // Response was not successfully parsed.
                Err(_) => return ErrorCode::InvalidMessage.into_errno(),
            }
        }
    }

    total_read as ssize_t
}
