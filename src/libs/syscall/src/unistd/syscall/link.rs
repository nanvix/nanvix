// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};

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

    ::syslog::warn!("link(): hard links not supported on VFS (oldpath={oldpath:?})");
    Err(Error::new(ErrorCode::OperationNotSupported, "hard links not supported on VFS"))
}
