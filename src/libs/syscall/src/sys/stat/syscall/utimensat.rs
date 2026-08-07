// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::stat::message::UpdateFileAccessTimeAtRequest,
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
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
use ::sysapi::time::timespec;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets file access and modification times.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the file.
/// - `times`: Access and modification times.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the `utimensat()` system call returns empty. Otherwise, it returns
/// an error.
///
pub fn utimensat(
    dirfd: i32,
    pathname: &str,
    times: &[timespec; 2],
    flags: i32,
) -> Result<(), Error> {
    ::syslog::trace!(
        "utimensat(): dirfd={:?}, pathname={:?}, times={:?}, flags={:?}",
        dirfd,
        pathname,
        times,
        flags
    );

    let pathname: alloc::borrow::Cow<'_, str> = crate::path::expand_path(pathname);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: UpdateFileAccessTimeAtRequest =
        UpdateFileAccessTimeAtRequest::new(dirfd, pathname.to_string(), flags, times)?;

    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    // Send request.
    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        ::syslog::warn!(
            "utimensat(): failed (dirfd={:?}, pathname={:?}, times={:?}, flags={:?}, \
             error_code={:?})",
            dirfd,
            pathname,
            times,
            flags,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => Err(Error::new(error_code, "utimensat() failed")),
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::warn!(
                    "utimensat(): failed to convert error code (dirfd={:?}, pathname={:?}, \
                     times={:?}, flags={:?}, error={:?})",
                    dirfd,
                    pathname,
                    times,
                    flags,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "utimensat() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::UpdateFileAccessTimeAtResponse => Ok(()),
            // Response was not successfully parsed.
            _ => {
                let reason: &str = "unexpected message header";
                ::syslog::warn!(
                    "utimensat(): failed (dirfd={:?}, pathname={:?}, times={:?}, flags={:?}, \
                     reason={:?})",
                    dirfd,
                    pathname,
                    times,
                    flags,
                    reason
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
