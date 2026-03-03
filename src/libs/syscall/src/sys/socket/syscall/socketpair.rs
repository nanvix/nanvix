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
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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
#[allow(unreachable_code)]
pub fn socketpair(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    socket_fds: &mut [c_int],
) -> Result<(), Error> {
    ::syslog::trace!("socketpair(): domain={:?}, type={:?}, protocol={:?}", domain, typ, protocol);

    #[cfg(feature = "standalone")]
    {
        let _ = (domain, typ, protocol, socket_fds);
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "socketpair not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Check if array of file descriptors has expected length.
    if socket_fds.len() != 2 {
        let reason: &str = "array of file descriptors must have length 2";
        ::syslog::error!("socketpair(): failed ({:?})", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Build request and send it.
    let request: Message = CreateSocketPairRequest::build(tid, domain, typ, protocol);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("socketpair(): failed ({:?})", error_code);
        Err(Error::new(error_code, "socketpair() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::CreateSocketPairResponse => {
                let response: CreateSocketPairResponse =
                    CreateSocketPairResponse::from_bytes(message.payload);

                // Store file descriptors.
                socket_fds[0] = response.sockfd_0;
                socket_fds[1] = response.sockfd_1;

                Ok(())
            },
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
