// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::stat::message::UpdateFileAccessTimeRequest,
    SystemCallMessage,
    SystemCallMessageKind,
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
/// Sets access and modification times of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `times`: Access and modification times.
///
/// # Returns
///
/// Upon successful completion, `futimens()` returns empty. Otherwise, it returns an error.
///
pub fn futimens(fd: RawFileDescriptor, times: &[timespec; 2]) -> Result<(), Error> {
    ::syslog::warn!("futimens(): fd={:?}, times={:?}", fd, times);
    let backend_fd: RawFileDescriptor = crate::fdtable::resolve_vfs(fd, "futimens")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let mut request: Message = UpdateFileAccessTimeRequest::build(
        tid,
        backend_fd,
        times,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!("futimens(): failed (fd={:?}, times={:?}, status={:?})", fd, times, {
            response.status
        });

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "futimens() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::warn!(
                    "futimens(): failed to parse error code (fd={:?}, times={:?}, error={:?})",
                    fd,
                    times,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "futimens() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::UpdateFileAccessTimeResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::syslog::warn!(
                    "futimens(): invalid response (fd={:?}, times={:?}, header={:?})",
                    fd,
                    times,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "futimens() failed"))
            },
        }
    }
}
