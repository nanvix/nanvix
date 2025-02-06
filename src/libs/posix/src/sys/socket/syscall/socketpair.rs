// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::socket::{
        message::{
            CreateSocketPairRequest,
            CreateSocketPairResponse,
        },
        AddressFamily,
        Protocol,
        SocketType,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

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
    ::nvx::log!("socketpair(): domain={:?}, type={:?}, protocol={:?}", domain, typ, protocol);
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Check if array of file descriptors has expected length.
    if socket_fds.len() != 2 {
        let reason: &str = "array of file descriptors must have length 2";
        ::nvx::log!("socketpair(): failed ({:?})", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Build request and send it.
    let request: Message = CreateSocketPairRequest::build(pid, domain, typ, protocol);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::nvx::log!("socketpair(): failed ({:?})", error_code);
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
