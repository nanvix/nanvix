// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::FileSpaceControlRequest,
    safe::RawFileDescriptor,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use sysapi::sys_types::off_t;

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
    ::syslog::error!("posix_fallocate(): fd={:?}, offset={:?}, len={:?}", fd, offset, len);

    // No-op for VFS file descriptors (FAT32 does not support pre-allocation).
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return Ok(());
        }
    }

    // In standalone mode, succeed as a no-op (no linuxd).
    #[cfg(feature = "standalone")]
    {
        let _ = (fd, offset, len);
        return Ok(());
    }

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
