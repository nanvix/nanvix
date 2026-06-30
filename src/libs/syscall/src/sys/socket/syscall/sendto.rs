// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::{
        message::{
            SendToSocketRequest,
            SendToSocketResponse,
        },
        sockaddr,
        socklen_t,
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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a datagram on a socket to an explicit destination address.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `buffer`: Buffer that holds the message to send.
/// - `flags`: Type of message transmission.
/// - `sockaddr`: Destination address of the message.
///
/// # Returns
///
/// On success, the number of bytes sent is returned. On error, an error is returned.
///
pub fn sendto(
    sockfd: c_int,
    buffer: &[u8],
    flags: c_int,
    sockaddr: &SocketAddr,
) -> Result<usize, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let (sockaddr, _socklen): (sockaddr, socklen_t) = From::<&SocketAddr>::from(sockaddr);

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd: c_int = crate::fdtable::resolve_socket(sockfd, "sendto")?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    // A datagram cannot be split across transfers, so it must fit in a single page-bounded push.
    if buffer.len() > SendToSocketRequest::MAX_DATA_SIZE {
        return Err(Error::new(
            ErrorCode::MessageTooLong,
            "datagram is too large for a single message",
        ));
    }

    // Build metadata-only request and send it via IPC message.
    let request: Message =
        SendToSocketRequest::build(tid, sockfd, buffer.len() as c_size_t, flags, &sockaddr);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Push the datagram payload via data chunk transfer.
    ::sys::kcall::ipc::__kcall_push(ProcessIdentifier::KERNEL, ThreadIdentifier::KERNEL, buffer)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "failed to send data through socket"))
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            Ok(message) => match message.header {
                SystemCallMessageHeader::SendToSocketResponse => {
                    let response: SendToSocketResponse =
                        SendToSocketResponse::from_bytes(message.payload);
                    Ok(response.count as usize)
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
