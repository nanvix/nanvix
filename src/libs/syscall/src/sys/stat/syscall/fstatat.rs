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
    let path: alloc::borrow::Cow<'_, str> = crate::path::expand_path(path);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: FileStatAtRequest = FileStatAtRequest::new(dirfd, path.to_string(), flag)?;

    let requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    for request in &requests {
        ::sys::kcall::ipc::__kcall_send(request)?;
    }

    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
}
