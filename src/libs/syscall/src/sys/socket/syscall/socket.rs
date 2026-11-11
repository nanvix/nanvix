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
        SocketFlags,
        SocketType,
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
/// Creates a socket with no socket creation flags.
///
/// # Parameters
///
/// - `domain`: Communication domain.
/// - `typ`: Socket type.
/// - `protocol`: Protocol.
///
/// # Returns
///
/// On success, the `socket()` function returns the file descriptor of the new socket. On error, it
/// returns an error.
///
pub fn socket(domain: AddressFamily, typ: SocketType, protocol: Protocol) -> Result<c_int, Error> {
    socket_with_flags(domain, typ, protocol, SocketFlags::default())
}

///
/// # Description
///
/// Creates a socket with socket creation flags. The file descriptor of the new socket is created
/// with the per-descriptor flags and the open file status flags that `flags` requests.
///
/// # Parameters
///
/// - `domain`: Communication domain.
/// - `typ`: Socket type.
/// - `protocol`: Protocol.
/// - `flags`: Socket creation flags.
///
/// # Returns
///
/// On success, the `socket_with_flags()` function returns the file descriptor of the new socket.
/// On error, it returns an error.
///
pub fn socket_with_flags(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    flags: SocketFlags,
) -> Result<c_int, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let mut request: Message = CreateSocketRequest::build(tid, domain, typ, protocol);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "failed to create socket")),
            Err(e) => Err(e),
        }
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.kind() {
                // Response was successfully parsed.
                SystemCallMessageKind::CreateSocketResponse => {
                    let response: CreateSocketResponse =
                        CreateSocketResponse::from_bytes(message.payload);
                    let remote_fd: c_int = response.sockfd;

                    // Under the flat namespace, `networkd` owns the endpoint (the remote fd) while
                    // `vfsd` allocates the application-visible flat descriptor that routes to it.
                    // The flat slot is recorded in the resolution cache so socket I/O reaches
                    // `networkd` directly by `remote_fd`.
                    if remote_fd < 0 {
                        Ok(remote_fd)
                    } else {
                        super::register_socket_slot(remote_fd, flags)
                    }
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
