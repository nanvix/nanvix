// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

use crate::fcntl;
use ::sys::error::Error;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::c_int,
    sys_types::mode_t,
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
/// Upon successful completion, the file descriptor of the file is returned. Otherwise, an error is
/// returned instead.
///
pub fn open(pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
    ::syslog::trace!("open(): pathname={:?}, flags={:?}, mode={:?}", pathname, flags, mode);

    fcntl::openat(AT_FDCWD, pathname, flags, mode)
}
