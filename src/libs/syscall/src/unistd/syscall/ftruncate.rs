// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::FileTruncateRequest,
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
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Truncates a file to a specified length.
///
/// # Parameters
///
/// `fd`: File descriptor.
/// `length`: New size of the file.
///
/// # Returns
///
/// Upon successful completion, `ftruncate()` returns empty. Otherwise, it returns an error.
///
pub fn ftruncate(fd: c_int, length: off_t) -> Result<(), Error> {
    ::syslog::debug!("ftruncate(): fd={}, length={}", fd, length);
    let backend_fd: c_int = crate::fdtable::resolve_vfs(fd, "ftruncate")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let mut request: Message = FileTruncateRequest::build(
        tid,
        backend_fd,
        length,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "ftruncate(): system call failed: fd={}, length={}, status={}",
            fd,
            length,
            { response.status }
        );

        // System call failed, parse error.
        match ErrorCode::try_from(response.status) {
            // System call failed, return error.
            Ok(error_code) => Err(Error::new(error_code, "system call failed")),
            // Invalid error code.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid error code")),
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::FileTruncateResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::warn!(
                    "ftruncate(): invalid response: fd={}, length={}, header={:?}",
                    fd,
                    length,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
