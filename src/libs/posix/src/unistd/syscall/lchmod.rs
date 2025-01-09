// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    sys::types::mode_t,
};
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the mode of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, it returns an error.
///
pub fn lchmod(path: &str, mode: mode_t) -> Result<(), Error> {
    ::nvx::log!("lchmod(): path = {:?}, mode = {:?}", path, mode);
    crate::fcntl::fchmodat(crate::fcntl::AT_FDCWD, path, mode, fcntl::AT_SYMLINK_NOFOLLOW)
}
