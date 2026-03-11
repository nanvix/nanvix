// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::FileTruncateRequest,
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
/// Truncates a file to a specified length.
///
/// # Parameters
///
/// `fd`: File descriptor.
/// `length`: New size of the file.
///
/// # Returns
///
/// Upon successful completion, `ftruncate()` returns empty. Otherwise, it returns an error.
///
pub fn ftruncate(fd: c_int, length: off_t) -> Result<(), Error> {
    ::syslog::debug!("ftruncate(): fd={}, length={}", fd, length);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_ftruncate(fd, length).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("ftruncate(): VFS ftruncate failed (fd={fd}, error={e})");
                Error::new(code, "vfs ftruncate failed")
            });
        }
        Err(Error::new(
            ErrorCode::OperationNotSupported,
            "ftruncate not available in standalone mode",
        ))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    ftruncate_linuxd(fd, length)
}

/// Forwards a `ftruncate` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn ftruncate_linuxd(fd: c_int, length: off_t) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = FileTruncateRequest::build(tid, fd, length);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "ftruncate(): system call failed: fd={}, length={}, status={}",
            fd,
            length,
            { response.status }
        );

        // System call failed, parse error.
        match ErrorCode::try_from(response.status) {
            // System call failed, return error.
            Ok(error_code) => Err(Error::new(error_code, "system call failed")),
            // Invalid error code.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid error code")),
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileTruncateResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::error!(
                    "ftruncate(): invalid response: fd={}, length={}, header={:?}",
                    fd,
                    length,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
