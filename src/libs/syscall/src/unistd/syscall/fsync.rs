// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::FileSyncRequest,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Synchronizes changes to a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned.
///
pub fn fsync(fd: c_int) -> Result<(), Error> {
    ::syslog::trace!("fsync(): fd={:?}", fd);
    let backend_fd: c_int = crate::fdtable::resolve_vfs(fd, "fsync")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message =
        FileSyncRequest::build(tid, backend_fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("fsync(): failed ({:?})", error_code);
        Err(Error::new(error_code, "fsync() failed"))
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileSyncResponse => Ok(()),
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
