// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::sys::stat::syscall::mkdirat;
use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::AT_FDCWD;
use ::sysapi::sys_types::mode_t;

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
#[allow(unreachable_code)]
pub fn mkdir(pathname: &str, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("mkdir(): pathname={pathname:?}, mode={mode:?}");

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_mkdir(pathname).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("mkdir(): VFS mkdir failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs mkdir failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    mkdirat(AT_FDCWD, pathname, mode)
}
