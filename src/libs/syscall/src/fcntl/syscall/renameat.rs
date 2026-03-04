// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::RenameAtRequest,
    message::MessagePartitioner,
    safe::RawFileDescriptor,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Renames a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the old file.
/// - `oldpath`:  Pathname of the old file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`:  Pathname of the new file.
///
/// # Returns
///
/// Upon successful completion, the `renameat()` system call returns empty. Otherwise, it returns an
/// error.
///
#[allow(unreachable_code)]
pub fn renameat(
    olddirfd: RawFileDescriptor,
    oldpath: &str,
    newdirfd: RawFileDescriptor,
    newpath: &str,
) -> Result<(), Error> {
    ::syslog::trace!(
        "renameat(): olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}",
        olddirfd,
        oldpath,
        newdirfd,
        newpath
    );

    // Route to the VFS if the old path belongs to an in-memory filesystem mount.
    #[cfg(feature = "memfs")]
    {
        let old_resolved: Option<::alloc::string::String> =
            ::nvx::vfs::fd::vfs_resolve_path(olddirfd, oldpath);
        let old_target: &str = old_resolved.as_deref().unwrap_or(oldpath);
        if ::nvx::vfs::fd::is_vfs_path(old_target) {
            let new_resolved: Option<::alloc::string::String> =
                ::nvx::vfs::fd::vfs_resolve_path(newdirfd, newpath);
            let new_target: &str = new_resolved.as_deref().unwrap_or(newpath);
            return ::nvx::vfs::fd::vfs_rename(old_target, new_target).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!(
                    "renameat(): VFS rename failed (oldpath={oldpath:?}, error={e})"
                );
                Error::new(code, "vfs rename failed")
            });
        }
    }

    // In standalone mode, non-VFS paths simply do not exist.
    #[cfg(feature = "standalone")]
    {
        return Err(Error::new(
            ErrorCode::NoSuchEntry,
            "renameat: path not found in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: RenameAtRequest = RenameAtRequest::new(olddirfd, oldpath, newdirfd, newpath)?;
    let requests: Vec<Message> = request.into_parts(tid)?;
    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "renameat(): failed (olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}, \
             error_code={:?})",
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "renameat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "renameat(): failed to parse error code (olddirfd={:?}, oldpath={:?}, \
                     newdirfd={:?}, newpath={:?}, error={:?})",
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "renameat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            LinuxDaemonMessageHeader::RenameAtResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::error!(
                    "renameat(): {:?} (olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}, \
                     header={:?})",
                    reason,
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
