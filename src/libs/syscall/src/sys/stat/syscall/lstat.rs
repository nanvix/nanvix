// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::sys;
use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::{
    AT_FDCWD,
    AT_SYMLINK_NOFOLLOW,
};
use ::sysapi::sys_stat;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file.
///
/// if pathname is a symbolic link, then it returns information about the link itself.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
#[allow(unreachable_code)]
pub fn lstat(pathname: &str, buf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("lstat(): pathname = {:?}, statbuf = {:?}", pathname, buf);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_stat(pathname, buf).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("lstat(): VFS lstat failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs lstat failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    sys::stat::fstatat(AT_FDCWD, pathname, buf, AT_SYMLINK_NOFOLLOW)
}
