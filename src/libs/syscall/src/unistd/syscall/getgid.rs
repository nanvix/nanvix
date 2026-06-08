// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::sys_types::gid_t;
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
/// Returns the group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getgid()` returns the group ID of the calling process. Otherwise, it
/// returns an error.
///
pub fn getgid() -> Result<gid_t, Error> {
    ::syslog::trace!("getgid()");

    // In standalone mode, return 0 (root).
    #[cfg(feature = "standalone")]
    return Ok(usize::from(::sys::pm::GroupIdentifier::ROOT) as gid_t);

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    getgid_linuxd()
}

/// Forwards a `getgid` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn getgid_linuxd() -> Result<gid_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it
    let request: Message = GetIdsRequest::build(tid, crate::LINUXD, ::sys::ipc::MessageType::Ikc);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not
    if response.status != 0 {
        ::syslog::warn!("getgid(): failed (tid={:?}, status={:?})", tid, { response.status });

        match ErrorCode::try_from(response.status) {
            // System call failed, return error
            Ok(error_code) => Err(Error::new(error_code, "getgid() failed")),
            // Invalid error code
            Err(_) => Err(Error::new(ErrorCode::TryAgain, "getgid() failed")),
        }
    } else {
        // System call succeeded, parse response
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed
            SystemCallMessageHeader::GetIdsResponse => {
                let response: GetIdsResponse = GetIdsResponse::from_bytes(message.payload);
                Ok(response.gid)
            },
            // Invalid response
            header => {
                ::syslog::warn!("getgid(): invalid response (tid={:?}, header={:?})", tid, header);
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
