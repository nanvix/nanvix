// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::select::message::{
        SelectRequest,
        SelectResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    kcall::pm,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_select::{
    fd_set,
    timeval,
    FD_SETSIZE,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Performs synchronous I/O multiplexing.
///
/// # Parameters
///
/// - `nfds`: Highest-numbered file descriptor plus one.
/// - `readfds`: Set of file descriptors to be checked for readability.
/// - `writefds`: Set of file descriptors to be checked for writability.
/// - `errorfds`: Set of file descriptors to be checked for errors.
///
/// # Return Value
///
/// On success, this function returns the number of file descriptors contained in the
/// three returned descriptor sets that are ready for I/O. On failure, an error code is
/// returned instead.
///
#[allow(unreachable_code)]
pub fn select(
    nfds: usize,
    readfds: Option<&mut fd_set>,
    writefds: Option<&mut fd_set>,
    errorfds: Option<&mut fd_set>,
    timeout: &Option<timeval>,
) -> Result<usize, Error> {
    ::syslog::trace!(
        "select(): nfds={:?}, readfds={:?}, writefds={:?}, errorfds={:?}, timeout={:?}",
        nfds,
        readfds,
        writefds,
        errorfds,
        timeout
    );

    if nfds > FD_SETSIZE {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "number of file descriptors exceeds maximum supported",
        ));
    }

    // In standalone mode, select is not available (no linuxd).
    #[cfg(feature = "standalone")]
    {
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "select not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = pm::__kcall_gettid()?;

    // Build the request and send it as a multi-part `SelectRequestPart` stream.
    let request: SelectRequest = SelectRequest::new(nfds, &readfds, &writefds, &errorfds, timeout)?;
    let requests: Vec<Message> =
        request.into_parts(tid, crate::LINUXD, ::sys::ipc::MessageType::Ikc)?;
    for request in &requests {
        ::sys::kcall::ipc::__kcall_send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "select(): failed (nfds={:?}, timeout={:?}, status={:?})",
            nfds,
            timeout,
            { response.status }
        );

        // System call failed, return error.
        match ErrorCode::try_from(response.status) {
            // Error was successfully parsed.
            Ok(error_code) => Err(Error::new(error_code, "select() failed")),
            // Error was not parsed.
            Err(error) => {
                ::syslog::warn!(
                    "select(): {error:?} (nfds={:?}, timeout={:?}, status={:?})",
                    nfds,
                    timeout,
                    { response.status }
                );
                Err(Error::new(ErrorCode::TryAgain, "select() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::SelectResponse => {
                let response: SelectResponse = SelectResponse::from_bytes(message.payload)?;

                for (fd_set, dest) in [
                    (response.readfds.as_ref(), readfds),
                    (response.writefds.as_ref(), writefds),
                    (response.errorfds.as_ref(), errorfds),
                ] {
                    if let (Some(src), Some(dest)) = (fd_set, dest) {
                        *dest = *src;
                    }
                }

                Ok(response.ready_fds as usize)
            },
            // Response was not parsed.
            header => {
                let reason: &'static str = "invalid response";
                ::syslog::warn!(
                    "select(): {reason} (header={header:?}, nfds={:?}, timeout={:?}, status={:?})",
                    nfds,
                    timeout,
                    { response.status }
                );
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
        }
    }
}
