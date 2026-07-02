// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    fcntl::{
        file_access_mode::{
            O_RDONLY,
            O_RDWR,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
        file_status_flags::O_APPEND,
    },
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        mode_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens the file whose pathname is the string pointed to by `pathname` and associates a stream
/// with it.
///
/// # Parameters
///
/// - `pathname`: Pointer to a null-terminated string containing the path of the file to open.
/// - `mode`: Pointer to a null-terminated string specifying the file access mode (`"r"`, `"w"`,
///   `"a"`, `"r+"`, `"w+"`, or `"a+"`).
///
/// # Returns
///
/// A pointer to a [`FILE`] object on success, or a null pointer on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `pathname` points to a valid, null-terminated string.
/// - `mode` points to a valid, null-terminated mode string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fopen.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fopen(pathname: *const c_char, mode: *const c_char) -> *mut FILE {
    extern "C" {
        fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int;
        fn malloc(size: c_size_t) -> *mut c_void;
        fn close(fd: c_int) -> c_int;
    }

    if pathname.is_null() || mode.is_null() {
        return core::ptr::null_mut();
    }

    // Parse the mode string.
    let first: u8 = *mode as u8;
    let second: u8 = *mode.add(1) as u8;
    let has_plus: bool = second == b'+' || (second != 0 && *mode.add(2) as u8 == b'+');

    let flags: c_int = match (first, has_plus) {
        (b'r', false) => O_RDONLY,
        (b'r', true) => O_RDWR,
        (b'w', false) => O_WRONLY | O_CREAT | O_TRUNC,
        (b'w', true) => O_RDWR | O_CREAT | O_TRUNC,
        (b'a', false) => O_WRONLY | O_CREAT | O_APPEND,
        (b'a', true) => O_RDWR | O_CREAT | O_APPEND,
        _ => return core::ptr::null_mut(),
    };

    let perm: mode_t = 0o666;

    // SAFETY: pathname is non-null, flags and permissions are valid.
    let fd: c_int = unsafe { open(pathname, flags, perm) };
    if fd < 0 {
        return core::ptr::null_mut();
    }

    // SAFETY: allocating memory for a FILE struct.
    let ptr: *mut c_void = unsafe { malloc(core::mem::size_of::<FILE>() as c_size_t) };
    if ptr.is_null() {
        // SAFETY: fd is a valid open file descriptor.
        unsafe { close(fd) };
        return core::ptr::null_mut();
    }

    let file: *mut FILE = ptr.cast::<FILE>();
    (*file).fd = fd;
    (*file).eof = 0;
    (*file).error = 0;
    (*file).ungetc_buf = -1;
    (*file).orientation = 0;

    file
}
