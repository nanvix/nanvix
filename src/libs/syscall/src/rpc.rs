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
    pm::{
        ProcessIdentifier,
        SigSet,
        SIG_SETMASK,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// Keeps signals blocked until a tagged bulk rendezvous is registered in the kernel.
pub(crate) struct SignalMaskGuard {
    previous: SigSet,
}

impl SignalMaskGuard {
    /// Blocks all catchable signals and retains the previous mask for atomic restoration.
    pub(crate) fn block_all() -> Result<Self, Error> {
        let blocked: SigSet = !0;
        let mut previous: SigSet = 0;
        unsafe {
            ::sys::kcall::pm::__kcall_sigprocmask(SIG_SETMASK, &blocked, &mut previous)?;
        }
        Ok(Self { previous })
    }

    /// Returns the signal mask that the kernel should restore after registration.
    pub(crate) const fn previous(&self) -> SigSet {
        self.previous
    }
}

impl Drop for SignalMaskGuard {
    fn drop(&mut self) {
        unsafe {
            let _result: Result<(), Error> = ::sys::kcall::pm::__kcall_sigprocmask(
                SIG_SETMASK,
                &self.previous,
                core::ptr::null_mut(),
            );
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Begins a correlated request for the calling thread.
pub(crate) fn begin_request(responder: ProcessIdentifier) -> Result<RequestToken, Error> {
    let tid = ::sys::kcall::pm::__kcall_gettid()?;
    RequestToken::allocate(tid, responder)
}

/// Sends one message as a new correlated request.
pub(crate) fn send_request(request: &mut Message) -> Result<RequestToken, Error> {
    let responder: ProcessIdentifier = request.destination.pid;
    let token: RequestToken = begin_request(responder)?;
    send_request_with_token(&token, request)?;
    Ok(token)
}

/// Sends all messages as parts of one new correlated request.
pub(crate) fn send_requests(requests: &mut [Message]) -> Result<RequestToken, Error> {
    let responder: ProcessIdentifier = requests
        .first()
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "request list is empty"))?
        .destination
        .pid;
    if requests.iter().any(|request| {
        let destination = request.destination;
        destination.pid != responder
    }) {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "multipart request has inconsistent destinations",
        ));
    }
    let token: RequestToken = begin_request(responder)?;
    for request in requests {
        send_request_with_token(&token, request)?;
    }
    Ok(token)
}

/// Sends one message with an existing request token.
pub(crate) fn send_request_with_token(
    token: &RequestToken,
    request: &mut Message,
) -> Result<(), Error> {
    token.identifier().write_to(request);
    ::sys::kcall::ipc::__kcall_send(request)
}

/// Receives the next response that matches `token`.
pub(crate) fn recv_response(token: &RequestToken) -> Result<Message, Error> {
    loop {
        match recv_response_interruptible(token) {
            Err(error) if error.code == ErrorCode::Interrupted => continue,
            result => return result,
        }
    }
}

/// Attempts to receive the next matching response, preserving `EINTR` for explicit arbitration.
pub(crate) fn recv_response_interruptible(token: &RequestToken) -> Result<Message, Error> {
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
