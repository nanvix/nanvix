// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    unistd::message::FileSyncRequest,
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
/// Synchronizes changes to a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned.
///
pub fn fsync(fd: c_int) -> Result<(), Error> {
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = FileSyncRequest::build(pid, fd);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::nvx::log!("fsync(): failed ({:?})", error_code);
        Err(Error::new(error_code, "fsync() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileSyncResponse => Ok(()),
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
