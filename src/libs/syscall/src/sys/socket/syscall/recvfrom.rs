// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::{
        message::{
            ReceiveFromSocketRequest,
            ReceiveFromSocketResponse,
        },
        SocketAddr,
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
    ipc::{
        Message,
        RequestToken,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Receives a datagram on a socket and reports the source address.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `buffer`: Buffer that stores the received message.
/// - `flags`: Type of message reception.
///
/// # Returns
///
/// On success, a tuple with the number of bytes received and the source address is returned. On
/// error, an error is returned.
///
pub fn recvfrom(
    sockfd: c_int,
    buffer: &mut [u8],
    flags: c_int,
) -> Result<(usize, SocketAddr), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd: c_int = crate::fdtable::resolve_socket(sockfd, "recvfrom")?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    // A datagram cannot be reassembled across transfers, so cap the request at a single
    // scatter/gather pull.
    let recv_len: usize = cmp::min(ReceiveFromSocketResponse::MAX_DATA_SIZE, buffer.len());

    // Build metadata-only request and send it via IPC message.
    let mut request: Message = ReceiveFromSocketRequest::build(tid, sockfd, recv_len as u32, flags);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Pull the datagram payload via data chunk transfer.
    let bytes_pulled: usize = ::sys::kcall::ipc::__kcall_pull_tagged(
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        &mut buffer[..recv_len],
        token.identifier(),
    )?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "failed to receive data on socket"))
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            Ok(message) => match message.header {
                SystemCallMessageHeader::ReceiveFromSocketResponse => {
                    let response: ReceiveFromSocketResponse =
                        ReceiveFromSocketResponse::from_bytes(message.payload);

                    let count: usize = response.count as usize;

                    // Validate the reported count against the requested length to avoid
                    // out-of-bounds slicing on a malformed or buggy response.
                    if count > recv_len {
                        return Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "response count exceeds requested length",
                        ));
                    }

                    // The bulk data was already delivered into the user buffer by the pull above,
                    // so the metadata count must match the bytes actually pulled.
                    if count != bytes_pulled {
                        return Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "response count does not match bytes pulled",
                        ));
                    }

                    // Convert source address.
                    let sockaddr: SocketAddr = SocketAddr::try_from(&response.sockaddr)?;

                    Ok((count, sockaddr))
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
