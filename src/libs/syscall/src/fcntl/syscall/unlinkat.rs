// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::Error;
use ::sysapi::ffi::c_int;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        fcntl::message::UnlinkAtRequest,
        message::MessagePartitioner,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::vec::Vec,
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
/// Unlinks a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the file.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the `unlinkat()` system call returns empty. Otherwise, it returns an
/// error.
///
#[allow(unreachable_code)]
pub fn unlinkat(dirfd: RawFileDescriptor, pathname: &str, flags: c_int) -> Result<(), Error> {
    ::syslog::trace!("unlinkat(): dirfd={}, pathname={}, flags={}", dirfd, pathname, flags);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_unlinkat(dirfd, pathname, flags).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("unlinkat(): VFS unlinkat failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs unlinkat failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    unlinkat_linuxd(dirfd, pathname, flags)
}

/// Forwards an `unlinkat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn unlinkat_linuxd(dirfd: RawFileDescriptor, pathname: &str, flags: c_int) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: UnlinkAtRequest = UnlinkAtRequest::new(dirfd, pathname, flags)?;
    let requests: Vec<Message> = request.into_parts(tid)?;
    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "unlinkat(): failed (dirfd={}, pathname={}, flags={}, error_code={})",
            dirfd,
            pathname,
            flags,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => Err(Error::new(error_code, "unlinkat() failed")),
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "unlinkat(): failed to parse error code (dirfd={:?}, pathname={:?}, \
                     flags={:?}, error={:?})",
                    dirfd,
                    pathname,
                    flags,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "unlinkat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::UnlinkAtResponse => Ok(()),
            // Response was not parsed.
            header => {
                ::syslog::error!(
                    "unlinkat(): failed to parse response (dirfd={:?}, pathname={:?}, flags={:?}, \
                     header={:?})",
                    dirfd,
                    pathname,
                    flags,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "unlinkat(): failed"))
            },
        }
    }
}
