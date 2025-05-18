// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::socket::{
        message::{
            GetSockNameRequest,
            GetSockNameResponse,
        },
        SocketAddr,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
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

///
/// # Description
///
/// Gets the name of the socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Location to store the address of the socket.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error number is returned.
///
pub fn getsockname(sockfd: c_int, sockaddr: &mut SocketAddr) -> Result<(), Error> {
    ::syslog::trace!("getsockname(): sockfd={:?}, sockaddr={:?}", sockfd, sockaddr);
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = GetSockNameRequest::build(pid, sockfd);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("getsockname(): failed ({:?})", error_code);
        Err(Error::new(error_code, "getsockname() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::GetSockNameResponse => {
                let response: GetSockNameResponse =
                    GetSockNameResponse::from_bytes(message.payload);

                // Copy address.
                *sockaddr = SocketAddr::try_from(&response.sockaddr)?;

                Ok(())
            },
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
