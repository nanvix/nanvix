// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    unistd::message::FileChdirRequest,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
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
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, the `fchdir()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn fchdir(fd: c_int) -> Result<(), Error> {
    ::syslog::trace!("fchdir(): fd={:?}", fd);

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it
    let request: Message = FileChdirRequest::build(pid, fd);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("fchdir(): failed (fd={:?}, error_code={:?})", fd, { response.status });
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => Err(Error::new(error_code, "fchdir() failed")),
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!("fchdir(): failed to convert error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "fchdir() failed"))
            },
        }
    } else {
        // System call succeeded.
        Ok(())
    }
}
