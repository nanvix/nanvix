// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    unistd::message::SymbolicLinkAtRequest,
    SystemCallMessage,
    SystemCallMessageHeader,
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
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `dirfd`: Directory file descriptor.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `symlinkat()` returns empty. Otherwise, it returns an error.
///
pub fn symlinkat(target: &str, dirfd: i32, linkpath: &str) -> Result<(), Error> {
    ::syslog::trace!(
        "symlinkat(): target={:?}, dirfd={:?}, linkpath={:?}",
        target,
        dirfd,
        linkpath
    );

    let target: alloc::borrow::Cow<'_, str> = crate::path::expand_path(target);
    let linkpath: alloc::borrow::Cow<'_, str> = crate::path::expand_path(linkpath);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: SymbolicLinkAtRequest =
        SymbolicLinkAtRequest::new(target.to_string(), dirfd, linkpath.to_string())?;

    let requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    // Send request.
    for request in &requests {
        ::sys::kcall::ipc::__kcall_send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "symlinkat(): failed (target={:?}, dirfd={:?}, linkpath={:?}, error_code={:?})",
            target,
            dirfd,
            linkpath,
            { response.status },
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "symlinkat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::warn!(
                    "symlinkat(): failed to parse error code (target={:?}, dirfd={:?}, \
                     linkpath={:?}, error={:?})",
                    target,
                    dirfd,
                    linkpath,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "symlinkat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::SymbolicLinkAtResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::syslog::warn!(
                    "symlinkat(): failed to parse response (target={:?}, dirfd={:?}, \
                     linkpath={:?}, header={:?})",
                    target,
                    dirfd,
                    linkpath,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "symlinkat(): failed"))
            },
        }
    }
}
