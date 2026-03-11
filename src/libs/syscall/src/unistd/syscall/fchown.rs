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
use ::sysapi::sys_types::{
    gid_t,
    uid_t,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::FileChownRequest,
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
/// Changes the owner and group of a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `owner`: Owner of the file.
/// - `group`: Group of the file.
///
/// # Returns
///
/// Upon successful completion, `fchown()` returns empty. Otherwise, it returns an error.
///
pub fn fchown(fd: RawFileDescriptor, owner: uid_t, group: gid_t) -> Result<(), Error> {
    ::syslog::trace!("fchown(): fd={:?}, owner={:?}, group={:?}", fd, owner, group);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return Ok(());
        }
        Err(Error::new(ErrorCode::OperationNotSupported, "fchown not available in standalone mode"))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fchown_linuxd(fd, owner, group)
}

/// Forwards a `fchown` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fchown_linuxd(fd: RawFileDescriptor, owner: uid_t, group: gid_t) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it
    let request: Message = FileChownRequest::build(tid, fd, owner, group);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "fchown(): failed (fd={:?}, owner={:?}, group={:?}, status={:?})",
            fd,
            owner,
            group,
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            // System call failed, return error.
            Ok(error_code) => Err(Error::new(error_code, "system call failed")),
            // Invalid error code.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid error code received")),
        }
    } else {
        // System call succeeded, parse response.
        let message = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileChownResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::error!(
                    "fchown(): invalid response (fd={:?}, owner={:?}, group={:?}, header={:?})",
                    fd,
                    owner,
                    group,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
