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
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
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
pub fn connect(sockfd: c_int, sockaddr: &SocketAddr) -> Result<(), Error> {
    ::syslog::trace!("connect(): fd={:?}, sockaddr={:?}", sockfd, sockaddr);

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let (sockaddr, socklen): (sockaddr, socklen_t) = From::<&SocketAddr>::from(sockaddr);

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd = crate::fdtable::resolve_socket(sockfd, "connect")?;

    // Build request and send it.
    let mut request: Message = ConnectSocketRequest::build(tid, sockfd, &sockaddr, socklen);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("connect(): failed ({:?})", error_code);
        Err(Error::new(error_code, "connect() failed"))
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::ConnectSocketResponse => {
                let _response: ConnectSocketResponse =
                    ConnectSocketResponse::from_bytes(message.payload);
                Ok(())
            },
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
