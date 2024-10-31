// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::{
        off_t,
        size_t,
        ssize_t,
    },
    unistd::message::{
        PartialReadRequest,
        PartialReadResponse,
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

pub fn pread(fd: i32, buffer: *mut u8, count: size_t, offset: off_t) -> ssize_t {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Check if buffer is invalid.
    if buffer.is_null() {
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Check if count is invalid.
    if count <= 0 {
        return ErrorCode::InvalidArgument.into_errno();
    }

    // Construct buffer from raw parts.
    let buffer: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buffer, count as usize) };

    let mut total_read: ssize_t = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let chunk_size: usize =
            cmp::min(PartialReadResponse::BUFFER_SIZE, buffer.len() - buffer_offset);

        // Build request and send it.
        let request: Message = PartialReadRequest::build(
            pid,
            fd,
            chunk_size as size_t,
            offset + buffer_offset as off_t,
        );
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
                    LinuxDaemonMessageHeader::PartialReadResponse => {
                        // Parse response.
                        let response: PartialReadResponse =
                            PartialReadResponse::from_bytes(message.payload);

                        // Copy response buffer to user buffer.
                        buffer[buffer_offset..buffer_offset + chunk_size]
                            .copy_from_slice(&response.buffer[..chunk_size]);
                        total_read += response.count;
                        buffer_offset += chunk_size;
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
