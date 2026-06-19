// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::FileAdvisoryInformationRequest,
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
use ::sysapi::ffi::c_int;
use sysapi::sys_types::off_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Provides advice about the use of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
/// - `advice`: Advice to provide.
///
/// # Returns
///
/// Upon success, `posix_fadvise()` empty. Otherwise, it returns an error.
///
pub fn posix_fadvise(
    fd: RawFileDescriptor,
    offset: off_t,
    len: off_t,
    advice: c_int,
) -> Result<(), Error> {
    ::syslog::trace!(
        "posix_fadvise(): fd={:?}, offset={:?}, len={:?}, advice={:?}",
        fd,
        offset,
        len,
        advice
    );
    let backend_fd: RawFileDescriptor = crate::fdtable::resolve_vfs(fd, "posix_fadvise")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = FileAdvisoryInformationRequest::build(
        tid,
        backend_fd,
        offset,
        len,
        advice,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "posix_fadvise(): failed (fd={:?}, offset={:?}, len={:?}, advice={:?}, status={:?})",
            fd,
            offset,
            len,
            advice,
            { response.status }
        );

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => Err(Error::new(error_code, "posix_fadvise() failed")),
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::warn!("posix_fadvise(): invalid error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "posix_fadvise(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileAdvisoryInformationResponse => Ok(()),
            header => {
                // Response was not successfully parsed.
                ::syslog::warn!(
                    "posix_fadvise(): unexpected message header (fd={:?}, offset={:?}, len={:?}, \
                     advice={:?}, header={:?})",
                    fd,
                    offset,
                    len,
                    advice,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header"))
            },
        }
    }
}
