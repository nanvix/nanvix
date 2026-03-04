// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::{
        message::{
            GetPeerNameRequest,
            GetPeerNameResponse,
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

///
/// # Description
///
/// Gets the name of the peer socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Location to store the address of the peer socket.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error number is returned.
///
#[allow(unreachable_code)]
pub fn getpeername(sockfd: c_int, sockaddr: &mut SocketAddr) -> Result<(), Error> {
    ::syslog::trace!("getpeername(): sockfd={:?}, sockaddr={:?}", sockfd, sockaddr);

    #[cfg(feature = "standalone")]
    {
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "getpeername not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = GetPeerNameRequest::build(tid, sockfd);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("getpeername(): failed ({:?})", error_code);
        Err(Error::new(error_code, "getpeername() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::GetPeerNameResponse => {
                let response: GetPeerNameResponse =
                    GetPeerNameResponse::from_bytes(message.payload);

                // Copy address and size.
                *sockaddr = SocketAddr::try_from(&response.sockaddr)?;

                Ok(())
            },
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
