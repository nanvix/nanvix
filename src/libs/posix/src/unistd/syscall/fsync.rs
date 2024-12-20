// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        FileSyncRequest,
        FileSyncResponse,
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

///
/// # Description
///
/// Synchronizes changes to a file.
///
/// # Parameters
///
/// - `fd: i32`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, a negative error code is returned.
///
pub fn fsync(fd: i32) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Build request and send it.
    let request: Message = FileSyncRequest::build(pid, fd);
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
            Err(e) => e.code.into_errno(),
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => {
                match message.header {
                    // Response was successfully parsed.
                    LinuxDaemonMessageHeader::FileSyncResponse => {
                        let response: FileSyncResponse =
                            FileSyncResponse::from_bytes(message.payload);
                        // Return system call result value.
                        response.ret
                    },
                    // Response was not successfully parsed.
                    _ => ErrorCode::InvalidMessage.into_errno(),
                }
            },
            // Response was not successfully parsed.
            Err(e) => e.code.into_errno(),
        }
    }
}
