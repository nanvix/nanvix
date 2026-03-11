// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::sys_stat;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        sys::stat::message::FileStatAtRequest,
    },
    ::alloc::{
        string::ToString,
        vec::Vec,
    },
    ::sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    },
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
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
#[allow(unreachable_code, unused_variables)]
pub fn fstatat(dirfd: i32, path: &str, buf: &mut sys_stat::stat, flag: i32) -> Result<(), Error> {
    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_stat(path, buf).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("fstatat(): VFS stat failed (path={path:?}, error={e})");
            ::sys::error::Error::new(code, "vfs stat failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fstatat_linuxd(dirfd, path, buf, flag)
}

/// Forwards a `fstatat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fstatat_linuxd(
    dirfd: i32,
    path: &str,
    buf: &mut sys_stat::stat,
    flag: i32,
) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: FileStatAtRequest = FileStatAtRequest::new(dirfd, path.to_string(), flag)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
}
