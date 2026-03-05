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
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
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
/// Returns the effective group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getegid()` returns the effective group ID of the calling process.
/// Otherwise, it returns an error.
///
pub fn getegid() -> Result<gid_t, Error> {
    ::syslog::trace!("getegid()");

    // In standalone mode, return 0 (root).
    #[cfg(feature = "standalone")]
    return Ok(usize::from(::sys::pm::GroupIdentifier::ROOT) as gid_t);

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    getegid_linuxd()
}

/// Forwards a `getegid` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn getegid_linuxd() -> Result<gid_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it
    let request: Message = GetIdsRequest::build(tid);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not
    if response.status != 0 {
        ::syslog::error!("getegid(): failed (tid={:?}, status={:?})", tid, { response.status });

        match ErrorCode::try_from(response.status) {
            // System call failed, return error
            Ok(error_code) => Err(Error::new(error_code, "getegid() failed")),
            // Invalid error code
            Err(_) => Err(Error::new(ErrorCode::TryAgain, "getegid() failed")),
        }
    } else {
        // System call succeeded, parse response
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed
            LinuxDaemonMessageHeader::GetIdsResponse => {
                let response: GetIdsResponse = GetIdsResponse::from_bytes(message.payload);
                Ok(response.egid)
            },
            // Invalid response
            header => {
                ::syslog::error!(
                    "getegid(): invalid response (tid={:?}, header={:?})",
                    tid,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
