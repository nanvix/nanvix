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

pub fn accept(sockfd: c_int) -> Result<(c_int, SocketAddr), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Address `networkd` by the descriptor it assigned to the listening socket: the caller passes a
    // flat descriptor, which resolves to the backend (`networkd`) fd.
    let backend_fd: c_int = crate::fdtable::resolve_socket(sockfd, "accept")?;

    // Build request and send it.
    let request: Message = AcceptSocketRequest::build(tid, backend_fd);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "failed to accept a connection")),
            Err(e) => Err(e),
        }
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                SystemCallMessageHeader::AcceptSocketResponse => {
                    let response: AcceptSocketResponse =
                        AcceptSocketResponse::from_bytes(message.payload);
                    let remote_fd: c_int = response.sockfd;
                    let sockaddr: SocketAddr = SocketAddr::try_from(&response.sockaddr)?;

                    // `networkd` created the accepted endpoint (the remote fd); `vfsd` allocates the
                    // application-visible flat descriptor that routes to it.
                    if remote_fd < 0 {
                        Ok((remote_fd, sockaddr))
                    } else {
                        let fd: c_int = super::register_socket_slot(remote_fd)?;
                        Ok((fd, sockaddr))
                    }
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
