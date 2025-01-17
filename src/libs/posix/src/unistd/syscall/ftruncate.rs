// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::types::off_t,
    unistd::message::FileTruncateRequest,
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
/// Truncates a file to a specified length.
///
/// # Parameters
///
/// `fd`: File descriptor.
/// `length`: New size of the file.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned.
///
pub fn ftruncate(fd: c_int, length: off_t) -> Result<(), Error> {
    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it.
    let request: Message = FileTruncateRequest::build(pid, fd, length);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::nvx::log!("ftruncate(): failed ({:?})", error_code);
        Err(Error::new(error_code, "ftruncate() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileTruncateResponse => Ok(()),
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
