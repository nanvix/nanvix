// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::uid_t,
    unistd::message::{
        GetIdsRequest,
        GetIdsResponse,
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
/// Returns the effective user ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `geteuid()` returns the effective user ID of the calling process.
/// Otherwise, it returns an error.
///
pub fn geteuid() -> Result<uid_t, Error> {
    ::nvx::trace!("geteuid()");

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it
    let request: Message = GetIdsRequest::build(pid);
    ::nvx::ipc::send(&request)?;

    // Receive response
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not
    if response.status != 0 {
        ::nvx::error!("geteuid(): failed (pid={:?}, status={:?})", pid, { response.status });

        match ErrorCode::try_from(response.status) {
            // System call failed, return error
            Ok(error_code) => Err(Error::new(error_code, "geteuid() failed")),
            // Invalid error code
            Err(_) => Err(Error::new(ErrorCode::TryAgain, "geteuid() failed")),
        }
    } else {
        // System call succeeded, parse response
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed
            LinuxDaemonMessageHeader::GetIdsResponse => {
                let response: GetIdsResponse = GetIdsResponse::from_bytes(message.payload);
                Ok(response.euid)
            },
            // Invalid response
            header => {
                ::nvx::error!("geteuid(): invalid response (pid={:?}, header={:?})", pid, header);
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
