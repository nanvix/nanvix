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
        PartialWriteRequest,
        PartialWriteResponse,
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

pub fn pwrite(fd: i32, buffer: *const u8, count: size_t, offset: off_t) -> ssize_t {
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
    let buffer: &[u8] = unsafe { ::core::slice::from_raw_parts(buffer, count as usize) };

    let mut total_written: ssize_t = 0;
    let mut buffer_offset: usize = 0;

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
        if let Err(e) = ::nvx::ipc::send(&request) {
            return e.code.into_errno();
        }

        // Receive response.
        let response: Message = match ::nvx::ipc::recv() {
            Ok(response) => response,
            Err(e) => return e.code.into_errno(),
        };

        // Check whether the system call succeeded or not.
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
                    LinuxDaemonMessageHeader::PartialWriteResponse => {
                        // Parse response.
                        let message: PartialWriteResponse =
                            PartialWriteResponse::from_bytes(message.payload);

                        // Update total written count.
                        total_written += message.count as ssize_t;
                        buffer_offset += message.count as usize;
                    },
                    // Response was not expected.
                    _ => return ErrorCode::InvalidMessage.into_errno(),
                },
                // Response was not parsed.
                Err(_) => return ErrorCode::InvalidMessage.into_errno(),
            }
        }
    }

    total_written
}
