// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::FileDataSyncRequest,
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Synchronizes the data of a file descriptor to disk.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `fdatasync()` returns empty. Otherwise, it returns an error.
///
pub fn fdatasync(fd: RawFileDescriptor) -> Result<(), Error> {
    ::syslog::trace!("fdatasync(): fd={:?}", fd);
    let backend_fd: RawFileDescriptor = crate::fdtable::resolve_vfs(fd, "fdatasync")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = FileDataSyncRequest::build(
        tid,
        backend_fd,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!("fdatasync(): fd={:?}, status={:?}", fd, { response.status });

        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "fdatasync failed"))
            },
            // Error code was not parsed.
            Err(_) => {
                // Return error code.
                Err(Error::new(ErrorCode::InvalidMessage, "fdatasync failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileDataSyncResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::warn!(
                    "fdatasync(): fd={:?}, status={:?}, header={:?}",
                    fd,
                    { response.status },
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "fdatasync failed to parse response"))
            },
        }
    }
}
