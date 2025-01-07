// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    fcntl,
    ffi::c_int,
    sys::types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `open()` system call opens the file specified by `pathname`.
///
/// # Parameters
///
/// - `pathname`: Pathname of the file to open.
/// - `flags`:    Flags to open the file.
/// - `mode`:     Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the `open()` system call returns a non-negative integer representing
/// the lowest numbered unused file descriptor. Otherwise, an error code is returned.
///
pub fn open(pathname: &str, flags: c_int, mode: mode_t) -> c_int {
    ::nvx::log!("open(): pathname={:?}, flags={:?}, mode={:?}", pathname, flags, mode);
    fcntl::openat(fcntl::AT_FDCWD, pathname, flags, mode)
}
