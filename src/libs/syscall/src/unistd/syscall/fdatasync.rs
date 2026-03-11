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
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::FileDataSyncRequest,
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

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fsync(fd).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("fdatasync(): VFS fdatasync failed (fd={fd}, error={e})");
                Error::new(code, "vfs fdatasync failed")
            });
        }
        Ok(())
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fdatasync_linuxd(fd)
}

/// Forwards a `fdatasync` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fdatasync_linuxd(fd: RawFileDescriptor) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = FileDataSyncRequest::build(tid, fd);
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
