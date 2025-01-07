// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    fcntl,
    ffi::c_int,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `link()` system call creates a new hard link to an existing file. If `newpath` exists, it
/// will not be overwritten. This new name may be used exactly as the old one for any operation;
/// both names refer to the same file and it is impossible to tell which name was the "original".
///
/// # Parameters
///
/// - `oladpath`: path to the file to be linked.
/// - `newpath`: path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, an error code is returned instead.
///
pub fn link(oldpath: &str, newpath: &str) -> c_int {
    ::nvx::log!("link(): oldpath = {:?}, newpath = {:?}", oldpath, newpath);
    unistd::linkat(fcntl::AT_FDCWD, oldpath, fcntl::AT_FDCWD, newpath, 0)
}
