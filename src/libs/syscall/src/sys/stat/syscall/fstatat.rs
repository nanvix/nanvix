// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::stat::message::FileStatAtRequest,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::sys::{
    error::Error,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_stat;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `fstatat()` system call obtains information about a file.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`: Path to the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn fstatat(dirfd: i32, path: &str, buf: &mut sys_stat::stat, flag: i32) -> Result<(), Error> {
    // Send request.
    fstatat_request(dirfd, path, flag)?;

    // Wait for response.
    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
}

///
/// # Description
///
/// This function sends a request to the daemon to execute the `fstatat()` system call.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`: Path to the file.
/// - `flag`: Flags.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
fn fstatat_request(dirfd: i32, path: &str, flag: i32) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: FileStatAtRequest = FileStatAtRequest::new(dirfd, path.to_string(), flag)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    Ok(())
}
