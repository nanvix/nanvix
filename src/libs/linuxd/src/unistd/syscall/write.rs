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
    unistd::message::{
        WriteRequest,
        WriteResponse,
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

pub fn write(fd: i32, buffer: *const u8, count: size_t) -> ssize_t {
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
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize = cmp::min(WriteRequest::BUFFER_SIZE, buffer.len() - offset);
        let mut chunk: [u8; WriteRequest::BUFFER_SIZE] = [0; WriteRequest::BUFFER_SIZE];
        chunk[..chunk_size].copy_from_slice(&buffer[offset..offset + chunk_size]);

        // Build request and send it.
        let request: Message = WriteRequest::build(pid, fd, chunk_size as size_t, chunk);
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
                    LinuxDaemonMessageHeader::WriteResponse => {
                        // Parse response.
                        let response: WriteResponse = WriteResponse::from_bytes(message.payload);

                        // Update total written count.
                        total_written += response.count;
                        offset += chunk_size;
                    },
                    _ => return ErrorCode::InvalidMessage.into_errno(),
                },
                // Response was not successfully parsed.
                Err(_) => return ErrorCode::InvalidMessage.into_errno(),
            }
        }
    }

    total_written
}
