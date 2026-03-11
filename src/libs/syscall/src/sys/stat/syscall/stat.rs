// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys;
use ::sys::error::Error;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    sys_stat,
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

    // Route to the VFS if the path matches an in-memory filesystem mount.
    #[cfg(feature = "memfs")]
    {
        // In standalone mode, VFS is the only filesystem. Route all stats to VFS
        // unconditionally — let VFS return proper errors for missing files.
        #[cfg(feature = "standalone")]
        {
            return ::nvx::vfs::fd::vfs_stat(pathname, statbuf).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!("stat(): VFS stat failed (pathname={pathname:?}, error={e})");
                Error::new(code, "vfs stat failed")
            });
        }

        #[cfg(not(feature = "standalone"))]
        if ::nvx::vfs::fd::is_vfs_path(pathname) {
            return ::nvx::vfs::fd::vfs_stat(pathname, statbuf).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!("stat(): VFS stat failed (pathname={pathname:?}, error={e})");
                Error::new(code, "vfs stat failed")
            });
        }
    }

    sys::stat::fstatat(AT_FDCWD, pathname, statbuf, 0)
}
