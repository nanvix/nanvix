// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        unistd::message::SymbolicLinkAtRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::{
        string::ToString,
        vec::Vec,
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
/// Creates a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `dirfd`: Directory file descriptor.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `symlinkat()` returns empty. Otherwise, it returns an error.
///
pub fn symlinkat(target: &str, dirfd: i32, linkpath: &str) -> Result<(), Error> {
    ::syslog::trace!(
        "symlinkat(): target={:?}, dirfd={:?}, linkpath={:?}",
        target,
        dirfd,
        linkpath
    );

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_symlinkat(target, dirfd, linkpath).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!(
                "symlinkat(): VFS symlinkat failed (linkpath={linkpath:?}, error={e})"
            );
            Error::new(code, "vfs symlinkat failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    symlinkat_linuxd(target, dirfd, linkpath)
}

/// Forwards a `symlinkat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn symlinkat_linuxd(target: &str, dirfd: i32, linkpath: &str) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: SymbolicLinkAtRequest =
        SymbolicLinkAtRequest::new(target.to_string(), dirfd, linkpath.to_string())?;

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
            "symlinkat(): failed (target={:?}, dirfd={:?}, linkpath={:?}, error_code={:?})",
            target,
            dirfd,
            linkpath,
            { response.status },
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "symlinkat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "symlinkat(): failed to parse error code (target={:?}, dirfd={:?}, \
                     linkpath={:?}, error={:?})",
                    target,
                    dirfd,
                    linkpath,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "symlinkat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::SymbolicLinkAtResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::syslog::error!(
                    "symlinkat(): failed to parse response (target={:?}, dirfd={:?}, \
                     linkpath={:?}, header={:?})",
                    target,
                    dirfd,
                    linkpath,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "symlinkat(): failed"))
            },
        }
    }
}
