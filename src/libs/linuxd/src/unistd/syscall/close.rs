// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        CloseRequest,
        CloseResponse,
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

pub fn close(fd: i32) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Build request and send it.
    let request: Message = CloseRequest::build(pid, fd);
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
            Ok(e) => e.into_errno(),
            Err(_) => ErrorCode::InvalidMessage.into_errno(),
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::CloseResponse => {
                    // Parse response.
                    let response: CloseResponse = CloseResponse::from_bytes(message.payload);

                    // Return result.
                    response.ret
                },
                // Response was not successfully parsed.
                _ => ErrorCode::InvalidMessage.into_errno(),
            },
            // Response was not successfully parsed.
            Err(_) => ErrorCode::InvalidMessage.into_errno(),
        }
    }
}
