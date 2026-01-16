// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
    },
};
use ::syslog::trace_syscall;

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
/// - `oldpath`: Path to the file to be linked.
/// - `newpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `link()` returns zero. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `oldpath` points to a valid null-terminated string.
/// - `newpath` points to a valid null-terminated string.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    unistd::bindings::linkat::linkat(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)
}
