// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::{
        message::{
            ConnectSocketRequest,
            ConnectSocketResponse,
        },
        sockaddr,
        socklen_t,
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
/// Connects a socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Address of the socket.
///
/// # Returns
///
/// The `connect()` function returns empty on success. On error, it returns an error.
///
#[allow(unreachable_code)]
pub fn connect(sockfd: c_int, sockaddr: &SocketAddr) -> Result<(), Error> {
    ::syslog::trace!("connect(): fd={:?}, sockaddr={:?}", sockfd, sockaddr);

    #[cfg(feature = "standalone")]
    {
        let _ = (sockfd, sockaddr);
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "connect not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let (sockaddr, socklen): (sockaddr, socklen_t) = From::<&SocketAddr>::from(sockaddr);

    // Build request and send it.
    let request: Message = ConnectSocketRequest::build(tid, sockfd, &sockaddr, socklen);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

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
                let _response: ConnectSocketResponse =
                    ConnectSocketResponse::from_bytes(message.payload);
                Ok(())
            },
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
