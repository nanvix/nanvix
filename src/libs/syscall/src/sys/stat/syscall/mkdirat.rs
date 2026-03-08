// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    safe::RawFileDescriptor,
    sys::stat::message::MakeDirectoryAtRequest,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::mode_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new directory relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the new directory.
/// - `mode`: Mode of the new directory.
///
/// # Returns
///
/// Upon successful completion, the `mkdirat()` system call returns empty. Otherwise, it returns an
/// error.
///
#[allow(unreachable_code)]
pub fn mkdirat(dirfd: RawFileDescriptor, pathname: &str, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("mkdirat(): dirfd={:?}, pathname={:?}, mode={:?}", dirfd, pathname, mode);

    // Route to the VFS if the path belongs to an in-memory filesystem mount.
    #[cfg(feature = "memfs")]
    {
        // In standalone mode the VFS is the only filesystem, so always route
        // there — even for paths that do not exist yet (`is_vfs_path` would
        // return false for a not-yet-created directory).
        #[cfg(feature = "standalone")]
        {
            return ::nvx::vfs::fd::vfs_mkdir(pathname).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("mkdirat(): VFS mkdir failed (pathname={pathname:?}, error={e})");
                Error::new(code, "vfs mkdir failed")
            });
        }

        #[cfg(not(feature = "standalone"))]
        if ::nvx::vfs::fd::is_vfs_path(pathname) {
            return ::nvx::vfs::fd::vfs_mkdir(pathname).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("mkdirat(): VFS mkdir failed (pathname={pathname:?}, error={e})");
                Error::new(code, "vfs mkdir failed")
            });
        }
    }

    // In standalone mode, reject non-VFS paths (no linuxd).
    #[cfg(feature = "standalone")]
    {
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "mkdirat not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: MakeDirectoryAtRequest =
        MakeDirectoryAtRequest::new(dirfd, pathname.to_string(), mode)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    // Send request.
    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "mkdirat(): failed (dirfd={:?}, pathname={:?}, mode={:?}, error_code={:?})",
            dirfd,
            pathname,
            mode,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "mkdirat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "mkdirat(): failed to parse error code (dirfd={:?}, pathname={:?}, mode={:?}, \
                     error={:?})",
                    dirfd,
                    pathname,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "mkdirat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            LinuxDaemonMessageHeader::MakeDirectoryAtResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::error!(
                    "mkdirat(): {:?} (dirfd={:?}, pathname={:?}, mode={:?}, header={:?})",
                    reason,
                    dirfd,
                    pathname,
                    mode,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
