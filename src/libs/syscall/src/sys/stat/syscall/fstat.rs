// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::message::FileStatRequest;
use ::sys::{
    error::Error,
    ipc::{
        Message,
        RequestToken,
    },
    pm::ThreadIdentifier,
};
use sysapi::sys_stat;

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
pub fn fstat(fd: i32, buf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("fstat(): fd={:?}", fd);

    // vfsd answers `fstat` for every descriptor from the slot it owns, addressed by the
    // caller-facing flat descriptor. It synthesizes the metadata of descriptors that it does not
    // serve I/O for: a console reports as a character device and a socket as a socket, each with
    // an identity that descriptors duplicated from it share.
    let backend_fd: i32 = crate::fdtable::resolve_stat(fd, "fstat")?;

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let mut message: Message =
        FileStatRequest::build(tid, backend_fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    let token: RequestToken = crate::rpc::send_request(&mut message)?;

    *buf = crate::sys::stat::syscall::fstatat_response(&token)?;

    Ok(())
}
