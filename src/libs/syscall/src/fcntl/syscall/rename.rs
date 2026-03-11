// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::fcntl::renameat;
use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::AT_FDCWD;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Renames a file.
///
/// # Parameters
///
/// - `oldpath`:  Pathname of the old file.
/// - `newpath`:  Pathname of the new file.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
#[allow(unreachable_code)]
pub fn rename(oldpath: &str, newpath: &str) -> Result<(), Error> {
    ::syslog::trace!("rename(): oldpath={oldpath:?}, newpath={newpath:?}");

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_rename(oldpath, newpath).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("rename(): VFS rename failed (oldpath={oldpath:?}, error={e})");
            Error::new(code, "vfs rename failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    renameat(AT_FDCWD, oldpath, AT_FDCWD, newpath)
}
