// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    unistd::message::ChangeDirectoryRequest,
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the current working directory.
///
/// # Parameters
///
/// - `path`: Pathname of the new working directory.
///
/// # Returns
///
/// Upon successful completion, the `chdir()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn chdir(path: &str) -> Result<(), Error> {
    ::syslog::trace!("chdir(): path={:?}", path);

    let path: alloc::borrow::Cow<'_, str> = crate::path::expand_path(path);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: ChangeDirectoryRequest = ChangeDirectoryRequest::new(&path)?;
    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;
    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!("chdir(): failed (path={:?}, error_code={:?})", path, { response.status });
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "chdir() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::warn!(
                    "chdir(): failed to parse error code (path={:?}, error={:?})",
                    path,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "chdir(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.kind() {
            SystemCallMessageKind::ChangeDirectoryResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::warn!("chdir(): {:?} (path={:?}, header={:?})", reason, path, header);
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
