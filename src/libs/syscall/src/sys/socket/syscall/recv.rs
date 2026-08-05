// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::message::{
        ReceiveSocketRequest,
        ReceiveSocketResponse,
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

pub fn recv(sockfd: i32, buffer: &mut [u8], flags: c_int) -> Result<usize, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Address networkd by the descriptor it assigned: the caller passes a flat descriptor.
    let sockfd: c_int = crate::fdtable::resolve_socket(sockfd, "recv")?;

    // Check if count is invalid.
    if buffer.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument, "buffer length is zero"));
    }

    // The payload is delivered out-of-band via a single scatter/gather pull, so cap the request at
    // a single round trip.
    let recv_len: usize = cmp::min(ReceiveSocketResponse::MAX_DATA_SIZE, buffer.len());

    // Build metadata-only request and send it via IPC message.
    let mut request: Message = ReceiveSocketRequest::build(tid, sockfd, recv_len as u32, flags);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Pull the payload via data chunk transfer directly into the user buffer.
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
                SystemCallMessageHeader::ReceiveSocketResponse => {
                    let response: ReceiveSocketResponse =
                        ReceiveSocketResponse::from_bytes(message.payload);

                    let count: usize = response.count as usize;

                    // Validate the reported count against the requested length to avoid
                    // out-of-bounds behavior on a malformed or buggy response.
                    if count > recv_len {
                        return Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "response count exceeds requested length",
                        ));
                    }

                    // The payload was already delivered into the user buffer by the pull above, so
                    // the metadata count must match the bytes actually pulled.
                    if count != bytes_pulled {
                        return Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "response count does not match bytes pulled",
                        ));
                    }

                    Ok(count)
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid response")),
        }
    }
}
