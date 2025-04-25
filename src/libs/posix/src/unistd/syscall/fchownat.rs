// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    message::MessagePartitioner,
    sys::types::{
        gid_t,
        uid_t,
    },
    unistd::message::FileChownAtRequest,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
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
    ::nvx::trace!(
        "fchownat(): dirfd={:?}, path={:?}, owner={:?}, group={:?}, flag={:?}",
        dirfd,
        path,
        owner,
        group,
        flag
    );

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    let request: FileChownAtRequest = FileChownAtRequest::new(dirfd, owner, group, flag, path)?;

    let requests: Vec<Message> = request.into_parts(pid)?;

    for request in requests {
        ::nvx::ipc::send(&request)?;
    }

    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "fchownat() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;

        match message.header {
            LinuxDaemonMessageHeader::FileChownAtResponse => Ok(()),
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
