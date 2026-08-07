// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Sends one message as a new correlated request.
pub(super) fn send_request(request: &mut Message) -> Result<RequestToken, Error> {
    let tid = ::sys::kcall::pm::__kcall_gettid()?;
    let responder: ProcessIdentifier = request.destination.pid;
    let token: RequestToken = RequestToken::allocate(tid, responder)?;
    token.identifier().write_to(request);
    ::sys::kcall::ipc::__kcall_send(request)?;
    Ok(token)
}

/// Receives the next response that matches `token`.
pub(super) fn recv_response(token: &RequestToken) -> Result<Message, Error> {
    loop {
        match recv_response_interruptible(token) {
            Err(error) if error.code == ErrorCode::Interrupted => continue,
            result => return result,
        }
    }
}

/// Attempts to receive the next matching response, preserving `EINTR` for explicit arbitration.
pub(super) fn recv_response_interruptible(token: &RequestToken) -> Result<Message, Error> {
    token.receive_response_with(
        ::sys::kcall::ipc::__kcall_recv,
        |identifier| {
            ::syslog::warn!(
                "recv_response(): dropping stale response (expected_request_id={}, request_id={})",
                token.identifier().raw(),
                identifier.raw()
            );
        },
        |identifier, expected, actual| {
            ::syslog::warn!(
                "recv_response(): dropping response from unexpected source (request_id={}, \
                 expected_source={:?}, source={:?})",
                identifier.raw(),
                expected,
                actual
            );
        },
    )
}
