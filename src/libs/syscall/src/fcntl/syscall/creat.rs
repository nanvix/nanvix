// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    fcntl::{
        self,
        OpenFlags,
    },
    ffi::c_int,
    sys::types::mode_t,
};
use ::sys::error::Error;

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
    fcntl::openat(
        fcntl::AT_FDCWD,
        filename,
        OpenFlags::O_CREAT | OpenFlags::O_RDWR | OpenFlags::O_TRUNC,
        mode,
    )
}
