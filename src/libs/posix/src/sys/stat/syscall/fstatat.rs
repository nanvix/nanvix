// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::stat::{
        message::FileStatAtRequest,
        stat,
    },
};
use ::alloc::{
    string::ToString,
    vec::Vec,
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
/// Upon successful completion, `0` is returned. Upon failure, a negative error code is returned
/// instead.
///
pub fn fstatat(dirfd: i32, path: &str, buf: &mut stat, flag: i32) -> i32 {
    // Send request.
    let status: i32 = fstatat_request(dirfd, path, flag);
    if status != 0 {
        return status;
    }

    // Wait for response.
    crate::sys::stat::syscall::fstatat_response(buf)
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
/// Upon successful completion, `0` is returned. Upon failure, a negative error code is returned
/// instead.
///
fn fstatat_request(dirfd: i32, path: &str, flag: i32) -> i32 {
    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    let request: FileStatAtRequest = match FileStatAtRequest::new(dirfd, path.to_string(), flag) {
        Ok(request) => request,
        Err(e) => return e.code.into_errno(),
    };

    let requests: Vec<Message> = match request.into_parts(pid) {
        Ok(requests) => requests,
        Err(e) => return e.code.into_errno(),
    };

    for request in requests {
        match ::nvx::ipc::send(&request) {
            Ok(_) => (),
            Err(e) => return e.code.into_errno(),
        }
    }

    0
}
