// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::sys;
use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::AT_FDCWD;
use ::sysapi::sys_stat;

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
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
#[allow(unreachable_code)]
pub fn stat(pathname: &str, statbuf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("stat(): pathname = {:?}", pathname);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_stat(pathname, statbuf).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("stat(): VFS stat failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs stat failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    sys::stat::fstatat(AT_FDCWD, pathname, statbuf, 0)
}
