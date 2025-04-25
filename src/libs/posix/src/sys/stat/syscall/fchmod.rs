// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::{
        stat::message::FileChmodRequest,
        types::mode_t,
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
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `fchmod()` returns empty. Otherwise, it returns an error.
///
pub fn fchmod(fd: RawFileDescriptor, mode: mode_t) -> Result<(), Error> {
    ::nvx::trace!("fchmod(): fd={:?}, mode={:o}", fd, mode);

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it
    let request: Message = FileChmodRequest::build(pid, fd, mode);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!("fchmod(): syscall failed (fd={:?}, mode={:o}, status={:?})", fd, mode, {
            response.status
        });
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => {
                ::nvx::error!(
                    "fchmod(): syscall failed (fd={:?}, mode={:o}, error_code={:?})",
                    fd,
                    mode,
                    error_code
                );
                Err(Error::new(error_code, "system call failed"))
            },
            Err(error) => {
                ::nvx::error!(
                    "fchmod(): syscall failed (fd={:?}, mode={:o}, error={:?})",
                    fd,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::InvalidMessage, "system call failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileChmodResponse => Ok(()),
            // Invalid response.
            header => {
                ::nvx::error!(
                    "fchmod(): invalid response (fd={:?}, mode={:o}, header={:?})",
                    fd,
                    mode,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
