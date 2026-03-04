// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::message::FileStatRequest;
use ::sys::{
    error::Error,
    ipc::Message,
    pm::ThreadIdentifier,
};
use sysapi::sys_stat;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file.
///
/// # Parameters
///
/// - `fd`: File descriptor of the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
#[allow(unreachable_code)]
pub fn fstat(fd: i32, buf: &mut sys_stat::stat) -> Result<(), Error> {
    // Route to the VFS if this is a VFS file descriptor.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fstat(fd, buf).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!("fstat(): VFS fstat failed (fd={fd}, error={e})");
                Error::new(code, "vfs fstat failed")
            });
        }
    }

    // In standalone mode, reject non-VFS fds (no linuxd).
    #[cfg(feature = "standalone")]
    {
        let _ = (fd, buf);
        return Err(Error::new(
            ::sys::error::ErrorCode::OperationNotSupported,
            "fstat not available in standalone mode",
        ));
    }

    // Send request.
    fstat_request(fd)?;

    // Wait for response.
    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
}

///
/// # Description
///
/// This function sends a request to the daemon to execute the `fstat()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
fn fstat_request(fd: i32) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let message: Message = FileStatRequest::build(tid, fd);

    ::sys::kcall::ipc::send(&message)
}
