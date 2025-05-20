// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::FileDataSyncRequest,
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Synchronizes the data of a file descriptor to disk.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `fdatasync()` returns empty. Otherwise, it returns an error.
///
pub fn fdatasync(fd: RawFileDescriptor) -> Result<(), Error> {
    ::syslog::trace!("fdatasync(): fd={:?}", fd);

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it.
    let request: Message = FileDataSyncRequest::build(pid, fd);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("fdatasync(): fd={:?}, status={:?}", fd, { response.status });

        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "fdatasync failed"))
            },
            // Error code was not parsed.
            Err(_) => {
                // Return error code.
                Err(Error::new(ErrorCode::InvalidMessage, "fdatasync failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileDataSyncResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::error!(
                    "fdatasync(): fd={:?}, status={:?}, header={:?}",
                    fd,
                    { response.status },
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "fdatasync failed to parse response"))
            },
        }
    }
}
