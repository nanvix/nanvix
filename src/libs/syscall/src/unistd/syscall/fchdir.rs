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
    crate::unistd::message::FileChdirRequest,
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
/// Changes the current working directory.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, the `fchdir()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn fchdir(fd: c_int) -> Result<(), Error> {
    ::syslog::trace!("fchdir(): fd={:?}", fd);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fchdir(fd).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("fchdir(): VFS fchdir failed (fd={fd}, error={e})");
                Error::new(code, "vfs fchdir failed")
            });
        }
        Ok(())
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fchdir_linuxd(fd)
}

/// Forwards a `fchdir` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fchdir_linuxd(fd: c_int) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it
    let request: Message = FileChdirRequest::build(tid, fd);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("fchdir(): failed (fd={:?}, error_code={:?})", fd, { response.status });
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => Err(Error::new(error_code, "fchdir() failed")),
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!("fchdir(): failed to convert error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "fchdir() failed"))
            },
        }
    } else {
        // System call succeeded.
        Ok(())
    }
}
