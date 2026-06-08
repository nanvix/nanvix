// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::FileSpaceControlRequest,
    safe::RawFileDescriptor,
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
use sysapi::sys_types::off_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Ensures that the file space is allocated for a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
///
/// # Returns
///
/// Upon success, `posix_fallocate()` empty. Otherwise, it returns an error.
///
pub fn posix_fallocate(fd: RawFileDescriptor, offset: off_t, len: off_t) -> Result<(), Error> {
    ::syslog::trace!("posix_fallocate(): fd={:?}, offset={:?}, len={:?}", fd, offset, len);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = FileSpaceControlRequest::build(
        tid,
        fd,
        offset,
        len,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    )?;
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "posix_fallocate(): failed (fd={:?}, offset={:?}, len={:?}, status={:?})",
            fd,
            offset,
            len,
            { response.status }
        );

        // System call failed, return error.
        match ErrorCode::try_from(response.status) {
            // Error was successfully parsed.
            Ok(error_code) => Err(Error::new(error_code, "posix_fallocate() failed")),
            // Error was not parsed.
            Err(error) => {
                ::syslog::warn!(
                    "posix_fallocate(): failed (fd={:?}, offset={:?}, len={:?}, error={:?})",
                    fd,
                    offset,
                    len,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "posix_fallocate() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileSpaceControlResponse => Ok(()),
            // Response was not parsed.
            header => {
                ::syslog::warn!(
                    "posix_fallocate(): invalid response (fd={:?}, offset={:?}, len={:?}, \
                     header={:?})",
                    fd,
                    offset,
                    len,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "posix_fallocate() failed"))
            },
        }
    }
}
