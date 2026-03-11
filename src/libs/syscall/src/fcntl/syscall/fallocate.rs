// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::Error;
use sysapi::sys_types::off_t;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        fcntl::message::FileSpaceControlRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::sys::{
        error::ErrorCode,
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
/// Ensures that the file space is allocated for a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
///
/// # Returns
///
/// Upon success, `posix_fallocate()` empty. Otherwise, it returns an error.
///
#[allow(unreachable_code)]
pub fn posix_fallocate(fd: RawFileDescriptor, offset: off_t, len: off_t) -> Result<(), Error> {
    ::syslog::trace!("posix_fallocate(): fd={:?}, offset={:?}, len={:?}", fd, offset, len);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fallocate(fd, offset, len).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!("posix_fallocate(): VFS fallocate failed (fd={fd:?}, error={e})");
                Error::new(code, "vfs fallocate failed")
            });
        }
        Ok(())
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    posix_fallocate_linuxd(fd, offset, len)
}

/// Forwards a `posix_fallocate` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn posix_fallocate_linuxd(fd: RawFileDescriptor, offset: off_t, len: off_t) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = FileSpaceControlRequest::build(tid, fd, offset, len)?;
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "posix_fallocate(): failed (fd={:?}, offset={:?}, len={:?}, status={:?})",
            fd,
            offset,
            len,
            { response.status }
        );

        // System call failed, return error.
        match ErrorCode::try_from(response.status) {
            // Error was successfully parsed.
            Ok(error_code) => Err(Error::new(error_code, "posix_fallocate() failed")),
            // Error was not parsed.
            Err(error) => {
                ::syslog::error!(
                    "posix_fallocate(): failed (fd={:?}, offset={:?}, len={:?}, error={:?})",
                    fd,
                    offset,
                    len,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "posix_fallocate() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileSpaceControlResponse => Ok(()),
            // Response was not parsed.
            header => {
                ::syslog::error!(
                    "posix_fallocate(): invalid response (fd={:?}, offset={:?}, len={:?}, \
                     header={:?})",
                    fd,
                    offset,
                    len,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "posix_fallocate() failed"))
            },
        }
    }
}
