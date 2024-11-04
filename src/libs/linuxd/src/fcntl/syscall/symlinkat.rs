// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
    message::MessagePartitioner,
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
};
use nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn symlinkat(target: &str, dirfd: i32, linkpath: &str) -> i32 {
    // Send request.
    let status: i32 = symlinkat_request(target, dirfd, linkpath);
    if status != 0 {
        return status;
    }

    // Wait for response.
    symlinkat_response()
}

fn symlinkat_request(target: &str, dirfd: i32, linkpath: &str) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    let request: SymbolicLinkAtRequest =
        match SymbolicLinkAtRequest::new(target.to_string(), dirfd, linkpath.to_string()) {
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

fn symlinkat_response() -> i32 {
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
                LinuxDaemonMessageHeader::SymbolicLinkAtResponse => {
                    // Parse response.
                    let response: SymbolicLinkAtResponse =
                        SymbolicLinkAtResponse::from_bytes(message.payload);

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
