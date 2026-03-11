// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::time::timespec;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        sys::stat::message::UpdateFileAccessTimeRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::sys::{
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
#[allow(unreachable_code)]
pub fn futimens(fd: RawFileDescriptor, times: &[timespec; 2]) -> Result<(), Error> {
    ::syslog::error!("futimens(): fd={:?}, times={:?}", fd, times);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return Ok(());
        }
        Err(Error::new(
            ErrorCode::OperationNotSupported,
            "futimens not available in standalone mode",
        ))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    futimens_linuxd(fd, times)
}

/// Forwards a `futimens` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn futimens_linuxd(fd: RawFileDescriptor, times: &[timespec; 2]) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = UpdateFileAccessTimeRequest::build(tid, fd, times);
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
