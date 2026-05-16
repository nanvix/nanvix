// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::sys_types::uid_t;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::{
            GetIdsRequest,
            GetIdsResponse,
        },
        SystemCallMessage,
        SystemCallMessageHeader,
    },
    ::sys::{
        error::ErrorCode,
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the user ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getuid()` returns the user ID of the calling process. Otherwise, it
/// returns an error.
///
pub fn getuid() -> Result<uid_t, Error> {
    ::syslog::trace!("getuid()");

    // In standalone mode, return 0 (root).
    #[cfg(feature = "standalone")]
    return Ok(usize::from(::sys::pm::UserIdentifier::ROOT) as uid_t);

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    getuid_linuxd()
}

/// Forwards a `getuid` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn getuid_linuxd() -> Result<uid_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it
    let request: Message = GetIdsRequest::build(tid, crate::LINUXD, ::sys::ipc::MessageType::Ikc);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not
    if response.status != 0 {
        ::syslog::warn!("getuid(): failed (tid={:?}, status={:?})", tid, { response.status });

        match ErrorCode::try_from(response.status) {
            // System call failed, return error
            Ok(error_code) => Err(Error::new(error_code, "getuid() failed")),
            // Invalid error code
            Err(_) => Err(Error::new(ErrorCode::TryAgain, "getuid() failed")),
        }
    } else {
        // System call succeeded, parse response
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed
            SystemCallMessageHeader::GetIdsResponse => {
                let response: GetIdsResponse = GetIdsResponse::from_bytes(message.payload);
                Ok(response.uid)
            },
            // Invalid response
            header => {
                ::syslog::warn!("getuid(): invalid response (tid={:?}, header={:?})", tid, header);
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
