// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::{
        message::{
            OpenAtRequest,
            OpenAtResponse,
        },
        mode_t,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::ffi;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn openat(dirfd: i32, pathname: &str, flags: ffi::c_int, mode: mode_t) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Build request and send it.
    let request: Message = match OpenAtRequest::build(pid, dirfd, pathname, flags, mode) {
        Ok(request) => request,
        Err(e) => return e.code.into_errno(),
    };
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
                LinuxDaemonMessageHeader::OpenAtResponse => {
                    // Parse response.
                    let response: OpenAtResponse = OpenAtResponse::from_bytes(message.payload);

                    // Return file descriptor.
                    response.ret
                },
                _ => ErrorCode::InvalidMessage.into_errno(),
            },
            // Failed to parse response.
            Err(e) => e.code.into_errno(),
        }
    }
}
