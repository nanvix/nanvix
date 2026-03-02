// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::unistd;
use ::sys::error::Error;
#[cfg(feature = "memfs")]
use ::sys::error::ErrorCode;
use ::sysapi::fcntl::atflags::AT_FDCWD;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new hard link to an existing file.
///
/// # Parameters
///
/// - `oladpath`: path to the file to be linked.
/// - `newpath`: path to the new file.
///
/// # Returns
///
/// Upon successful completion, `link()` returns empty. Otherwise, it returns an error.
///
pub fn link(oldpath: &str, newpath: &str) -> Result<(), Error> {
    ::syslog::trace!("link(): oldpath = {:?}, newpath = {:?}", oldpath, newpath);

    // FAT32 does not support hard links.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_path(oldpath) {
            ::syslog::error!("link(): hard links not supported on VFS (oldpath={oldpath:?})");
            return Err(Error::new(
                ErrorCode::OperationNotSupported,
                "hard links not supported on VFS",
            ));
        }
    }

    unistd::linkat(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)
}
