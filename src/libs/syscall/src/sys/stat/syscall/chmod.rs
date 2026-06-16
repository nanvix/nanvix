// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::fchmodat;
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
    fchmodat(AT_FDCWD, path, mode, 0)
}
