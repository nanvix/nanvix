// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::{
        message::{
            AcceptSocketRequest,
            AcceptSocketResponse,
        },
        SocketAddr,
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
pub fn accept(sockfd: c_int) -> Result<(c_int, SocketAddr), Error> {
    #[cfg(feature = "standalone")]
    {
        let _ = sockfd;
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "accept not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = AcceptSocketRequest::build(tid, sockfd);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "failed to accept a connection")),
            Err(e) => Err(e),
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::AcceptSocketResponse => {
                    let response: AcceptSocketResponse =
                        AcceptSocketResponse::from_bytes(message.payload);

                    Ok((response.sockfd, SocketAddr::try_from(&response.sockaddr)?))
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
