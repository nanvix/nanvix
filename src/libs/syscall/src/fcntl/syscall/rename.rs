// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fcntl::{
    renameat,
    AT_FDCWD,
};
use ::sys::error::Error;

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
pub fn rename(oldpath: &str, newpath: &str) -> Result<(), Error> {
    ::syslog::trace!("rename(): oldpath={oldpath:?}, newpath={newpath:?}");
    renameat(AT_FDCWD, oldpath, AT_FDCWD, newpath)
}
