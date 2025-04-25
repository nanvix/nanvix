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
/// Upon successful completion, `ftruncate()` returns empty. Otherwise, it returns an error.
///
pub fn ftruncate(fd: c_int, length: off_t) -> Result<(), Error> {
    ::nvx::debug!("ftruncate(): fd={}, length={}", fd, length);

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it.
    let request: Message = FileTruncateRequest::build(pid, fd, length);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!(
            "ftruncate(): system call failed: fd={}, length={}, status={}",
            fd,
            length,
            { response.status }
        );

        // System call failed, parse error.
        match ErrorCode::try_from(response.status) {
            // System call failed, return error.
            Ok(error_code) => Err(Error::new(error_code, "system call failed")),
            // Invalid error code.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid error code")),
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileTruncateResponse => Ok(()),
            // Invalid response.
            header => {
                ::nvx::error!(
                    "ftruncate(): invalid response: fd={}, length={}, header={:?}",
                    fd,
                    length,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
