// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        SeekRequest,
        SeekResponse,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn lseek(fd: i32, offset: i64, whence: i32) -> i64 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno() as i64,
    };

    // Build request and send it.
    let request: Message = SeekRequest::build(pid, fd, offset, whence);
    if let Err(e) = ::nvx::ipc::send(&request) {
        return e.code.into_errno() as i64;
    }

    // Receive response.
    let response: Message = match ::nvx::ipc::recv() {
        Ok(response) => response,
        Err(e) => return e.code.into_errno() as i64,
    };

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(e) => e.into_errno() as i64,
            Err(_) => ErrorCode::InvalidMessage.into_errno() as i64,
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::SeekResponse => {
                    // Parse response.
                    let response: SeekResponse = SeekResponse::from_bytes(message.payload);

                    response.offset
                },
                // Response was not successfully parsed.
                _ => ErrorCode::InvalidMessage.into_errno() as i64,
            },
            // Response was not successfully parsed.
            Err(_) => ErrorCode::InvalidMessage.into_errno() as i64,
        }
    }
}
