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
use ::sysapi::sys_types::mode_t;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        sys::stat::message::MakeDirectoryAtRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::{
        string::ToString,
        vec::Vec,
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

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_mkdir(pathname).map_err(|e| {
            let code: ErrorCode = e.into();
            ::syslog::error!("mkdirat(): VFS mkdir failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs mkdir failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    mkdirat_linuxd(dirfd, pathname, mode)
}

/// Forwards a `mkdirat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn mkdirat_linuxd(dirfd: RawFileDescriptor, pathname: &str, mode: mode_t) -> Result<(), Error> {
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
