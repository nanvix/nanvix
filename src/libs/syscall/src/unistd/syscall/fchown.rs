// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::FileChownRequest,
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::{
    gid_t,
    uid_t,
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

    // In standalone mode, only VFS file descriptors should be routed to vfsd.
    #[cfg(feature = "standalone")]
    if !crate::is_vfs_fd(fd) {
        ::syslog::warn!("fchown(): bad file descriptor fd={fd} in standalone mode");
        return Err(Error::new(
            ErrorCode::BadFile,
            "fchown: fd is not a VFS fd in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it
    let request: Message = FileChownRequest::build(
        tid,
        fd,
        owner,
        group,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
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
        let message = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileChownResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::warn!(
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
