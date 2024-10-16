// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        RenameAtRequest,
        RenameAtResponse,
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

pub fn renameat(olddirfd: i32, oldpath: &str, newdirfd: i32, newpath: &str) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Build request and send it.
    let request: Message = match RenameAtRequest::build(pid, olddirfd, oldpath, newdirfd, newpath) {
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
            Ok(message) => match message.header {
                LinuxDaemonMessageHeader::RenameAtResponse => {
                    // Parse response.
                    let response: RenameAtResponse = RenameAtResponse::from_bytes(message.payload);

                    // Return result.
                    response.ret
                },
                _ => ErrorCode::InvalidMessage.into_errno(),
            },
            _ => ErrorCode::InvalidMessage.into_errno(),
        }
    }
}
