// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    safe::RawFileDescriptor,
    sys::stat::message::MakeDirectoryAtRequest,
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
use ::sysapi::sys_types::mode_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new directory relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the new directory.
/// - `mode`: Mode of the new directory.
///
/// # Returns
///
/// Upon successful completion, the `mkdirat()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn mkdirat(dirfd: RawFileDescriptor, pathname: &str, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("mkdirat(): dirfd={:?}, pathname={:?}, mode={:?}", dirfd, pathname, mode);

    let pathname: alloc::borrow::Cow<'_, str> = crate::path::expand_path(pathname);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: MakeDirectoryAtRequest =
        MakeDirectoryAtRequest::new(dirfd, pathname.to_string(), mode)?;

    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    // Send request.
    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "mkdirat(): failed (dirfd={:?}, pathname={:?}, mode={:?}, error_code={:?})",
            dirfd,
            pathname,
            mode,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "mkdirat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::warn!(
                    "mkdirat(): failed to parse error code (dirfd={:?}, pathname={:?}, mode={:?}, \
                     error={:?})",
                    dirfd,
                    pathname,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "mkdirat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.kind() {
            SystemCallMessageKind::MakeDirectoryAtResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::warn!(
                    "mkdirat(): {:?} (dirfd={:?}, pathname={:?}, mode={:?}, header={:?})",
                    reason,
                    dirfd,
                    pathname,
                    mode,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
