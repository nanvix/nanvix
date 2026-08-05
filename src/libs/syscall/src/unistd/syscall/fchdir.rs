// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd::message::FileChdirRequest;
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
use ::sysapi::ffi::c_int;

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
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, the `fchdir()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn fchdir(fd: c_int) -> Result<(), Error> {
    ::syslog::trace!("fchdir(): fd={:?}", fd);
    let backend_fd: c_int = crate::fdtable::resolve_vfs(fd, "fchdir")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it
    let mut request: Message =
        FileChdirRequest::build(tid, backend_fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!("fchdir(): failed (fd={:?}, error_code={:?})", fd, { response.status });
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => Err(Error::new(error_code, "fchdir() failed")),
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::warn!("fchdir(): failed to convert error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "fchdir() failed"))
            },
        }
    } else {
        // System call succeeded.
        Ok(())
    }
}
