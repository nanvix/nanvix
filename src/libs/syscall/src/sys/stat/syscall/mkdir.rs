// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::syscall::mkdirat;
use ::sys::error::Error;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new directory.
///
/// # Parameters
///
/// - `pathname`: Pathname of the new directory.
/// - `mode`: Mode of the new directory.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn mkdir(pathname: &str, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("mkdir(): pathname={pathname:?}, mode={mode:?}");

    // Route to the VFS if the path matches an in-memory filesystem mount.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_path(pathname) {
            return ::nvx::vfs::fd::vfs_mkdir(pathname).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!("mkdir(): VFS mkdir failed (pathname={pathname:?}, error={e})");
                Error::new(code, "vfs mkdir failed")
            });
        }
    }

    mkdirat(AT_FDCWD, pathname, mode)
}
