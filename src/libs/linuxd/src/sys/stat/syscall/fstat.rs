// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::{
    self,
    message::FileStatRequest,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file.
///
/// # Parameters
///
/// - `fd`: File descriptor of the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, a negative error code is returned
/// instead.
///
pub fn fstat(fd: i32, buf: &mut stat::stat) -> i32 {
    // Send request.
    let status: i32 = fstat_request(fd);
    if status != 0 {
        return status;
    }

    // Wait for response.
    crate::sys::stat::syscall::fstatat_response(buf)
}

///
/// # Description
///
/// This function sends a request to the daemon to execute the `fstat()` system call.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, a negative error code is returned
/// instead.
///
fn fstat_request(fd: i32) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    let message: Message = FileStatRequest::build(pid, fd);

    match ::nvx::ipc::send(&message) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
