// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::{
    self,
    message::FileStatRequest,
};
use ::sys::{
    error::Error,
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
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn fstat(fd: i32, buf: &mut stat::stat) -> Result<(), Error> {
    // Send request.
    fstat_request(fd)?;

    // Wait for response.
    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
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
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
fn fstat_request(fd: i32) -> Result<(), Error> {
    let pid: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    let message: Message = FileStatRequest::build(pid, fd);

    ::sys::kcall::ipc::send(&message)
}
