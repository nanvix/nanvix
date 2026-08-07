// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::UnlinkAtRequest,
    message::MessagePartitioner,
    safe::RawFileDescriptor,
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
pub fn unlinkat(dirfd: RawFileDescriptor, pathname: &str, flags: c_int) -> Result<(), Error> {
    ::syslog::trace!("unlinkat(): dirfd={}, pathname={}, flags={}", dirfd, pathname, flags);

    let pathname: alloc::borrow::Cow<'_, str> = crate::path::expand_path(pathname);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: UnlinkAtRequest = UnlinkAtRequest::new(dirfd, &pathname, flags)?;
    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;
    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
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
                ::syslog::warn!(
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
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::UnlinkAtResponse => Ok(()),
            // Response was not parsed.
            header => {
                ::syslog::warn!(
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
