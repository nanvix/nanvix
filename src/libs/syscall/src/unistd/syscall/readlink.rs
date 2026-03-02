// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::unistd;
use ::sys::error::Error;
#[cfg(feature = "memfs")]
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    sys_types::c_ssize_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the value of a symbolic link.
///
/// # Parameters
///
/// - `path`: The path to the symbolic link.
/// - `buf`: Storage location for the value of the symbolic link.
///
/// # Returns
///
/// Upon successful completion, `readlink()` returns the number of bytes read. Otherwise, it returns
/// an error.
///
pub fn readlink(path: &str, buf: &mut [u8]) -> Result<c_ssize_t, Error> {
    ::syslog::trace!("readlinkat(): path={path:?}, buf.len={}", buf.len());

    // FAT32 does not support symbolic links.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_path(path) {
            ::syslog::error!("readlink(): symlinks not supported on VFS (path={path:?})");
            return Err(Error::new(
                ErrorCode::OperationNotSupported,
                "symbolic links not supported on VFS",
            ));
        }
    }

    unistd::readlinkat(AT_FDCWD, path, buf)
}
