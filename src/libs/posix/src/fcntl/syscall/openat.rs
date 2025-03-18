// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        OpenAtRequest,
        OpenAtResponse,
    },
    ffi::c_int,
    message::MessagePartitioner,
    sys::types::mode_t,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn openat(dirfd: i32, pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: OpenAtRequest = OpenAtRequest::new(dirfd, pathname, flags, mode)?;
    let requests: Vec<Message> = request.into_parts(pid)?;
    for request in requests {
        ::nvx::ipc::send(&request)?;
    }

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::nvx::error!("openat(): failed (error={})", error_code);
        Err(Error::new(error_code, "openat() failed"))
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                LinuxDaemonMessageHeader::OpenAtResponse => {
                    // Parse response.
                    let response: OpenAtResponse = OpenAtResponse::from_bytes(message.payload);

                    // Return file descriptor.
                    Ok(response.ret)
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
        }
    }
}
