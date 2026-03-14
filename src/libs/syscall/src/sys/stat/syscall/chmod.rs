// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use crate::sys::stat::fchmodat;
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
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `path`:  Pathname of the file.
/// - `mode`:  Mode.
///
/// # Returns
///
/// Upon successful completion, the `fchmodat()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn chmod(path: &str, mode: mode_t) -> Result<(), Error> {
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_chmod(path, mode).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            Error::new(code, "vfs chmod failed")
        })
    }

    #[cfg(not(feature = "standalone"))]
    fchmodat(AT_FDCWD, path, mode, 0)
}
