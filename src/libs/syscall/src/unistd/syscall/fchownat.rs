// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    unistd::message::FileChownAtRequest,
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        gid_t,
        uid_t,
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

    let path: alloc::borrow::Cow<'_, str> = crate::path::expand_path(path);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: FileChownAtRequest = FileChownAtRequest::new(dirfd, owner, group, flag, &path)?;

    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
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
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;

        match message.kind() {
            SystemCallMessageKind::FileChownAtResponse => Ok(()),
            header => {
                ::syslog::warn!(
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
