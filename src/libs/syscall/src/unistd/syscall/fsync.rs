// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::FileSyncRequest,
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
    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fsync(fd).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("fsync(): VFS fsync failed (fd={fd}, error={e})");
                Error::new(code, "vfs fsync failed")
            });
        }
        Ok(())
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fsync_linuxd(fd)
}

/// Forwards a `fsync` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fsync_linuxd(fd: c_int) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = FileSyncRequest::build(tid, fd);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("fsync(): failed ({:?})", error_code);
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
