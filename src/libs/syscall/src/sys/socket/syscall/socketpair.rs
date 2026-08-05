// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    netinet::in_::Protocol,
    sys::socket::{
        message::{
            CreateSocketPairRequest,
            CreateSocketPairResponse,
        },
        AddressFamily,
        SocketType,
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
/// Creates a pair of connected sockets.
///
/// # Parameters
///
/// - `domain`: Communication domain.
/// - `typ`: Socket type.
/// - `protocol`: Protocol.
/// - `socket_fds`: Array where the file descriptors of the sockets will be stored.
///
/// # Returns
///
/// The `socketpair()` function returns empty on success. On error, it returns an error.
///
pub fn socketpair(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    socket_fds: &mut [c_int],
) -> Result<(), Error> {
    ::syslog::trace!("socketpair(): domain={:?}, type={:?}, protocol={:?}", domain, typ, protocol);

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Check if array of file descriptors has expected length.
    if socket_fds.len() != 2 {
        let reason: &str = "array of file descriptors must have length 2";
        ::syslog::warn!("socketpair(): failed ({:?})", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Build request and send it.
    let mut request: Message = CreateSocketPairRequest::build(tid, domain, typ, protocol);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("socketpair(): failed ({:?})", error_code);
        Err(Error::new(error_code, "socketpair() failed"))
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::CreateSocketPairResponse => {
                let response: CreateSocketPairResponse =
                    CreateSocketPairResponse::from_bytes(message.payload);

                let remote_fd_0: c_int = response.sockfd_0;
                let remote_fd_1: c_int = response.sockfd_1;

                // `networkd` owns both endpoints (the remote fds); `vfsd` allocates the
                // application-visible flat descriptors that route to them. If either registration
                // fails, the other endpoint is closed so a half-registered pair never strands an
                // endpoint on networkd.
                if remote_fd_0 < 0 || remote_fd_1 < 0 {
                    socket_fds[0] = remote_fd_0;
                    socket_fds[1] = remote_fd_1;
                    return Ok(());
                }
                let fd0: c_int = match super::register_socket_slot(remote_fd_0) {
                    Ok(fd0) => fd0,
                    Err(e) => {
                        // The first registration failed and already closed `remote_fd_0`; the
                        // second endpoint was never registered, so close it directly on
                        // networkd to avoid stranding it.
                        super::close_networkd_endpoint(remote_fd_1);
                        return Err(e);
                    },
                };
                let fd1: c_int = match super::register_socket_slot(remote_fd_1) {
                    Ok(fd1) => fd1,
                    Err(e) => {
                        // Roll back the already-registered first slot; closing it releases the
                        // vfsd slot and forwards the endpoint close to networkd.
                        let _ = crate::unistd::close(fd0);
                        return Err(e);
                    },
                };
                socket_fds[0] = fd0;
                socket_fds[1] = fd1;

                Ok(())
            },
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
