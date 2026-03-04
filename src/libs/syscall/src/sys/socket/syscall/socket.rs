// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    netinet::in_::Protocol,
    sys::socket::{
        message::{
            CreateSocketRequest,
            CreateSocketResponse,
        },
        AddressFamily,
        SocketType,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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

#[allow(unreachable_code)]
pub fn socket(domain: AddressFamily, typ: SocketType, protocol: Protocol) -> Result<c_int, Error> {
    #[cfg(feature = "standalone")]
    {
        let _ = (domain, typ, protocol);
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "socket not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = CreateSocketRequest::build(tid, domain, typ, protocol);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "failed to create socket")),
            Err(e) => Err(e),
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::CreateSocketResponse => {
                    let response: CreateSocketResponse =
                        CreateSocketResponse::from_bytes(message.payload);

                    // Return system call result.
                    Ok(response.sockfd)
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
