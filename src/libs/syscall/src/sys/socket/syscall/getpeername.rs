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
    SystemCallMessage,
    SystemCallMessageHeader,
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
pub fn getpeername(sockfd: c_int, sockaddr: &mut SocketAddr) -> Result<(), Error> {
    ::syslog::trace!("getpeername(): sockfd={:?}, sockaddr={:?}", sockfd, sockaddr);

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd = crate::fdtable::resolve_socket(sockfd, "getpeername")?;

    // Build request and send it.
    let mut request: Message = GetPeerNameRequest::build(tid, sockfd);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("getpeername(): failed ({:?})", error_code);
        Err(Error::new(error_code, "getpeername() failed"))
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::GetPeerNameResponse => {
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
