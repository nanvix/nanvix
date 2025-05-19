// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::socket::{
        message::{
            ConnectSocketRequest,
            ConnectSocketResponse,
        },
        sockaddr,
        socklen_t,
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
/// Connects a socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Address of the socket.
/// - `len`: Size of the address.
///
/// # Returns
///
/// The `connect()` function returns the file descriptor of the socket on success. On error, it returns an error.
///
pub fn connect(sockfd: c_int, sockaddr: &sockaddr, len: socklen_t) -> Result<c_int, Error> {
    ::syslog::trace!("connect(): fd={:?}, sockaddr={:?}, len={:?}", sockfd, sockaddr, len);
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = ConnectSocketRequest::build(pid, sockfd, sockaddr.clone(), len);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("connect(): failed ({:?})", error_code);
        Err(Error::new(error_code, "connect() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::ConnectSocketResponse => {
                let response: ConnectSocketResponse =
                    ConnectSocketResponse::from_bytes(message.payload);
                Ok(response.sockfd)
            },
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
