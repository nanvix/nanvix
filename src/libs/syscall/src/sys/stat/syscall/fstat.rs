// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::message::FileStatRequest;
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

    // Route by the descriptor's resolved backend. Both vfsd-served objects and
    // console descriptors are answered by vfsd from the slot it owns; the console reports as a
    // character device with a stable, dup-shared identity synthesized by vfsd.
    let backend_fd: i32 = {
        use crate::fdtable::{
            resolve_result,
            Route,
        };
        match resolve_result(fd)? {
            // A vfsd-served object or a console descriptor: vfsd answers `fstat` from the slot
            // it owns, addressed by the caller-facing flat descriptor. For a console this is the
            // slot number, not the stream number used to route I/O — the slot is the operand,
            // so a `dup`'d console descriptor shares its source's character-device identity.
            Some(res) if matches!(res.route, Route::Vfs | Route::Console) => fd,
            // Sockets and unroutable descriptors have no stat here.
            _ => {
                ::syslog::warn!("fstat(): bad file descriptor fd={fd}");
                return Err(Error::new(ErrorCode::BadFile, "fstat: fd is not a VFS fd"));
            },
        }
    };

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let mut message: Message =
        FileStatRequest::build(tid, backend_fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    let token: RequestToken = crate::rpc::send_request(&mut message)?;

    *buf = crate::sys::stat::syscall::fstatat_response(&token)?;

    Ok(())
}
