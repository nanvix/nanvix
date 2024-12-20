// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        MakeDirectoryAtRequest,
        MakeDirectoryAtResponse,
    },
    message::MessagePartitioner,
    sys::types::mode_t,
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

pub fn mkdirat(dirfd: i32, pathname: &str, mode: mode_t) -> i32 {
    // Send request.
    let status: i32 = mkdirat_request(dirfd, pathname, mode);
    if status != 0 {
        return status;
    }

    // Wait for response.
    mkdirat_response()
}

fn mkdirat_request(dirfd: i32, pathname: &str, mode: mode_t) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    let request: MakeDirectoryAtRequest =
        match MakeDirectoryAtRequest::new(dirfd, pathname.to_string(), mode) {
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

fn mkdirat_response() -> i32 {
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
                LinuxDaemonMessageHeader::MakeDirectoryAtResponse => {
                    // Parse response.
                    let response: MakeDirectoryAtResponse =
                        MakeDirectoryAtResponse::from_bytes(message.payload);

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
