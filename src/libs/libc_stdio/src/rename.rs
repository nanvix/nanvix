// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Renames the file `old` to `new`, delegating to `renameat` relative to the current working
/// directory.
///
/// # Parameters
///
/// - `old`: Path of the existing file.
/// - `new`: New path for the file.
///
/// # Returns
///
/// Zero on success, or -1 on failure with `errno` set.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. Both `old` and `new` must point to
/// valid, null-terminated strings.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int {
    extern "C" {
        fn renameat(
            olddirfd: c_int,
            oldpath: *const c_char,
            newdirfd: c_int,
            newpath: *const c_char,
        ) -> c_int;
    }

    // SAFETY: old and new are valid, null-terminated strings.
    unsafe { renameat(AT_FDCWD, old, AT_FDCWD, new) }
}
