// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::{
        message::ShutdownSocketRequest,
        Shutdown,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn shutdown(sockfd: c_int, how: Shutdown) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = ShutdownSocketRequest::build(tid, sockfd, how);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "failed to shutdown socket")),
            Err(e) => Err(e),
        }
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                SystemCallMessageHeader::ShutdownSocketResponse => Ok(()),
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
