// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::stat::message::{
        UpdateFileAccessTimeAtRequest,
        UpdateFileAccessTimeAtResponse,
    },
    time::timespec,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn utimensat(dirfd: i32, pathname: &str, times: [timespec; 2], flags: i32) -> i32 {
    // Send request.
    let status: i32 = utimensat_request(dirfd, pathname, times, flags);
    if status != 0 {
        return status;
    }

    // Wait for response.
    utimensat_response()
}

fn utimensat_request(dirfd: i32, pathname: &str, times: [timespec; 2], flags: i32) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    let request: UpdateFileAccessTimeAtRequest =
        match UpdateFileAccessTimeAtRequest::new(dirfd, pathname.to_string(), flags, times) {
            Ok(request) => request,
            Err(e) => return e.code.into_errno(),
        };

    let requests: Vec<Message> = match request.into_parts(pid) {
        Ok(requests) => requests,
        Err(e) => return e.code.into_errno(),
    };

    // Send request.
    for request in requests {
        match ::nvx::ipc::send(&request) {
            Ok(_) => (),
            Err(e) => return e.code.into_errno(),
        }
    }

    0
}

fn utimensat_response() -> i32 {
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
                LinuxDaemonMessageHeader::UpdateFileAccessTimeAtResponse => {
                    // Parse response.
                    let response: UpdateFileAccessTimeAtResponse =
                        UpdateFileAccessTimeAtResponse::from_bytes(message.payload);

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
