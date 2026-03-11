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

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::syslog::error!("link(): hard links not supported on VFS (oldpath={oldpath:?})");
        Err(Error::new(ErrorCode::OperationNotSupported, "hard links not supported on VFS"))
    }

    #[cfg(not(feature = "standalone"))]
    unistd::linkat(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)
}
