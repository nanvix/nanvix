// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        gid_t,
        uid_t,
    },
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        unistd::message::FileChownAtRequest,
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
/// Changes the owner and group of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `owner`: Owner of the file.
/// - `group`: Group of the file.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, `fchownat()` returns empty. Otherwise, it returns an error code.
///
pub fn fchownat(
    dirfd: c_int,
    path: &str,
    owner: uid_t,
    group: gid_t,
    flag: c_int,
) -> Result<(), Error> {
    ::syslog::trace!(
        "fchownat(): dirfd={:?}, path={:?}, owner={:?}, group={:?}, flag={:?}",
        dirfd,
        path,
        owner,
        group,
        flag
    );

    // In standalone mode, forward to VFS.
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_fchownat(dirfd, path, owner, group, flag).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!(
                "fchownat(): VFS fchownat failed (dirfd={dirfd:?}, path={path:?}, error={e})"
            );
            Error::new(code, "vfs fchownat failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fchownat_linuxd(dirfd, path, owner, group, flag)
}

/// Forwards a `fchownat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fchownat_linuxd(
    dirfd: c_int,
    path: &str,
    owner: uid_t,
    group: gid_t,
    flag: c_int,
) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: FileChownAtRequest = FileChownAtRequest::new(dirfd, owner, group, flag, path)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "fchownat(): failed (dirfd={:?}, path={:?}, owner={:?}, group={:?}, flag={:?}, \
             error_code={:?})",
            dirfd,
            path,
            owner,
            group,
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
            LinuxDaemonMessageHeader::FileChownAtResponse => Ok(()),
            header => {
                ::syslog::error!(
                    "fchownat(): failed to parse response (dirfd={:?}, path={:?}, owner={:?}, \
                     group={:?}, flag={:?}, header={:?})",
                    dirfd,
                    path,
                    owner,
                    group,
                    flag,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
            },
        }
    }
}
