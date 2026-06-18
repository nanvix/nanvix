// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Associates a stream with the already-open file descriptor `fd`. The `mode` string must be
/// compatible with the mode the descriptor was opened with; it is validated for a recognized
/// first character but otherwise advisory, since a [`FILE`] in this library is a thin wrapper
/// around the descriptor.
///
/// # Parameters
///
/// - `fd`: An open file descriptor to wrap.
/// - `mode`: Pointer to a null-terminated file access mode string (`"r"`, `"w"`, `"a"`, optionally
///   followed by `"+"` and/or `"b"`).
///
/// # Returns
///
/// A pointer to a [`FILE`] object on success, or a null pointer on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `fd` is a valid, open file descriptor.
/// - `mode` points to a valid, null-terminated mode string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fdopen.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE {
    extern "C" {
        fn malloc(size: c_size_t) -> *mut c_void;
    }

    if fd < 0 || mode.is_null() {
        return core::ptr::null_mut();
    }

    // Validate the access mode's first character.
    match *mode as u8 {
        b'r' | b'w' | b'a' => {},
        _ => return core::ptr::null_mut(),
    }

    // SAFETY: allocating memory for a FILE struct.
    let ptr: *mut c_void = unsafe { malloc(core::mem::size_of::<FILE>() as c_size_t) };
    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    let file: *mut FILE = ptr.cast::<FILE>();
    (*file).fd = fd;
    (*file).eof = 0;
    (*file).error = 0;
    (*file).ungetc_buf = -1;

    file
}
