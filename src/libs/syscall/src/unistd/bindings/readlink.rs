// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::c_char,
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the value of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the symbolic link.
/// - `buf`: Buffer to store the value of the symbolic link.
/// - `bufsize`: Size of the buffer.
///
/// # Returns
///
/// Upon successful completion, `readlink()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `buf` points to a valid memory location of `bufsize` bytes.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readlink(
    path: *const c_char,
    buf: *mut c_char,
    bufsize: c_size_t,
) -> c_ssize_t {
    ::syslog::trace!("readlink(): path={path:?}, buf={buf:?}, bufsize={bufsize}");
    unistd::bindings::readlinkat::readlinkat(AT_FDCWD, path, buf, bufsize)
}
