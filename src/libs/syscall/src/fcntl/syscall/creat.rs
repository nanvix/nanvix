// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    fcntl,
    safe::OpenFlags,
};
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
/// The `creat()` system call creates a new file or rewrites an existing file.
///
/// # Parameters
///
/// - `pathname`: Pathname of the file to open.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the file descriptor of the file is returned. Otherwise, an error is
/// returned instead.
///
pub fn creat(filename: &str, mode: mode_t) -> Result<c_int, Error> {
    ::syslog::trace!("creat(): pathname={filename:?}, mode={mode:?}");
    let flags: c_int = OpenFlags::read_write()
        .set_create(true)
        .set_truncate(true)
        .into();
    fcntl::openat(AT_FDCWD, filename, flags, mode)
}
