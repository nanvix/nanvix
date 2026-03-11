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
        message::MessagePartitioner,
        unistd::message::FileAccessAtRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::vec::Vec,
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
/// Checks the accessibility of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `mode`:  Accessibility check mode.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, `faccessat()` returns empty. Otherwise, it returns an error code.
///
pub fn faccessat(dirfd: c_int, path: &str, mode: c_int, flag: c_int) -> Result<(), Error> {
    ::syslog::trace!(
        "faccessat(): dirfd={:?}, path={:?}, mode={:?}, flag={:?}",
        dirfd,
        path,
        mode,
        flag
    );

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_access(path).map_err(|e| {
            let code: ErrorCode = e.into();
            ::syslog::error!("faccessat(): VFS access failed (path={path:?}, error={e})");
            Error::new(code, "vfs access failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    faccessat_linuxd(dirfd, path, mode, flag)
}

/// Forwards a `faccessat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn faccessat_linuxd(dirfd: c_int, path: &str, mode: c_int, flag: c_int) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: FileAccessAtRequest = FileAccessAtRequest::new(dirfd, path, mode, flag)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    for request in requests {
        ::sys::kcall::ipc::send(&request)?;
    }

    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "faccessat(): failed (dirfd={:?}, path={:?}, mode={:?}, flag={:?}, error_code={:?})",
            dirfd,
            path,
            mode,
            flag,
            { response.status },
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "failed")),
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "failed to parse error code")),
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;

        match message.header {
            LinuxDaemonMessageHeader::FileAccessAtResponse => Ok(()),
            header => {
                ::syslog::error!(
                    "faccessat(): failed to parse response (dirfd={:?}, path={:?}, mode={:?}, \
                     flag={:?}, header={:?})",
                    dirfd,
                    path,
                    mode,
                    flag,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
            },
        }
    }
}
