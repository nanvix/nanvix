// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::message::{
        SendSocketRequest,
        SendSocketResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::cmp;
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
/// Sends data on a connected socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `buffer`: Buffer that holds the message to send.
/// - `flags`: Type of message transmission.
///
/// # Returns
///
/// On success, the number of bytes sent is returned. On error, an error is returned.
///
pub fn send(sockfd: c_int, buffer: &[u8], flags: c_int) -> Result<usize, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd: c_int = crate::fdtable::resolve_socket(sockfd, "send")?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    // The payload travels out-of-band via a scatter/gather push, so cap the request at one page.
    // A stream socket may transfer fewer bytes than requested, so the caller is expected to
    // resubmit the remainder on a short send.
    let send_len: usize = cmp::min(SendSocketRequest::MAX_DATA_SIZE, buffer.len());

    // Build metadata-only request and send it via IPC message.
    let request: Message = SendSocketRequest::build(tid, sockfd, send_len as c_size_t, flags);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Push the payload via data chunk transfer.
    ::sys::kcall::ipc::__kcall_push(
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        &buffer[..send_len],
    )?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "failed to send data through socket"))
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            Ok(message) => match message.header {
                SystemCallMessageHeader::SendSocketResponse => {
                    let response: SendSocketResponse =
                        SendSocketResponse::from_bytes(message.payload);
                    Ok(response.count as usize)
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
