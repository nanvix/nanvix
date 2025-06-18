// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::stat::message::UpdateFileAccessTimeRequest,
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
use ::sysapi::time::timespec;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets access and modification times of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `times`: Access and modification times.
///
/// # Returns
///
/// Upon successful completion, `futimens()` returns empty. Otherwise, it returns an error.
///
pub fn futimens(fd: RawFileDescriptor, times: &[timespec; 2]) -> Result<(), Error> {
    ::syslog::error!("futimens(): fd={:?}, times={:?}", fd, times);

    let pid: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    // Build request and send it.
    let request: Message = UpdateFileAccessTimeRequest::build(pid, fd, times);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("futimens(): failed (fd={:?}, times={:?}, status={:?})", fd, times, {
            response.status
        });

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "futimens() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::error!(
                    "futimens(): failed to parse error code (fd={:?}, times={:?}, error={:?})",
                    fd,
                    times,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "futimens() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::UpdateFileAccessTimeResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::syslog::error!(
                    "futimens(): invalid response (fd={:?}, times={:?}, header={:?})",
                    fd,
                    times,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "futimens() failed"))
            },
        }
    }
}
