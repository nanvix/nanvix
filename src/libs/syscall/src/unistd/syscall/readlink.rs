// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::unistd;
use ::sys::error::Error;
#[cfg(feature = "standalone")]
use ::sys::error::ErrorCode;
#[cfg(not(feature = "standalone"))]
use ::sysapi::fcntl::atflags::AT_FDCWD;
use ::sysapi::sys_types::c_ssize_t;

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

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::syslog::error!("readlink(): symlinks not supported on VFS (path={path:?})");
        Err(Error::new(ErrorCode::OperationNotSupported, "symbolic links not supported on VFS"))
    }

    #[cfg(not(feature = "standalone"))]
    unistd::readlinkat(AT_FDCWD, path, buf)
}
