// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        GetIdsRequest,
        GetIdsResponse,
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
    pm::ProcessIdentifier,
};
use ::sysapi::sys_types::uid_t;

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

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it
    let request: Message = GetIdsRequest::build(pid);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not
    if response.status != 0 {
        ::syslog::error!("getuid(): failed (pid={:?}, status={:?})", pid, { response.status });

        match ErrorCode::try_from(response.status) {
            // System call failed, return error
            Ok(error_code) => Err(Error::new(error_code, "getuid() failed")),
            // Invalid error code
            Err(_) => Err(Error::new(ErrorCode::TryAgain, "getuid() failed")),
        }
    } else {
        // System call succeeded, parse response
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed
            LinuxDaemonMessageHeader::GetIdsResponse => {
                let response: GetIdsResponse = GetIdsResponse::from_bytes(message.payload);
                Ok(response.uid)
            },
            // Invalid response
            header => {
                ::syslog::error!("getuid(): invalid response (pid={:?}, header={:?})", pid, header);
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
