// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    unistd::message::FileAccessAtRequest,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks the accessibility of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `mode`:  Accessibility check mode.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, `faccessat()` returns empty. Otherwise, it returns an error code.
///
pub fn faccessat(dirfd: c_int, path: &str, mode: c_int, flag: c_int) -> Result<(), Error> {
    ::syslog::trace!(
        "faccessat(): dirfd={:?}, path={:?}, mode={:?}, flag={:?}",
        dirfd,
        path,
        mode,
        flag
    );

    let path: alloc::borrow::Cow<'_, str> = crate::path::expand_path(path);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: FileAccessAtRequest = FileAccessAtRequest::new(dirfd, &path, mode, flag)?;

    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => {
                ::syslog::warn!(
                    "faccessat(): failed (dirfd={:?}, path={:?}, mode={:?}, flag={:?}, \
                     error_code={:?})",
                    dirfd,
                    path,
                    mode,
                    flag,
                    error_code,
                );
                Err(Error::new(error_code, "faccessat() failed"))
            },
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "failed to parse error code")),
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;

        match message.kind() {
            SystemCallMessageKind::FileAccessAtResponse => Ok(()),
            header => {
                ::syslog::warn!(
                    "faccessat(): failed to parse response (dirfd={:?}, path={:?}, mode={:?}, \
                     flag={:?}, header={:?})",
                    dirfd,
                    path,
                    mode,
                    flag,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
            },
        }
    }
}
