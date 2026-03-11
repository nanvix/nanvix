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
pub fn chmod(_path: &str, _mode: mode_t) -> Result<(), Error> {
    // In standalone mode, this operation is not supported.
    // TODO: https://github.com/nanvix/nanvix/issues/1606
    #[cfg(feature = "standalone")]
    {
        Ok(())
    }

    #[cfg(not(feature = "standalone"))]
    fchmodat(AT_FDCWD, _path, _mode, 0)
}
