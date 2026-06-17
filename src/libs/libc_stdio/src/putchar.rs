// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// File descriptor for standard output.
const STDOUT_FD: c_int = 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the character `c` (converted to `unsigned char`) to standard output.
///
/// # Parameters
///
/// - `c`: The character to write, passed as a [`c_int`].
///
/// # Returns
///
/// The character written as an `unsigned char` cast to [`c_int`], or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it calls an external write function.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/putchar.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn putchar(c: c_int) -> c_int {
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    let byte: u8 = c as u8;
    // SAFETY: byte is a valid 1-byte value on the stack.
    let ret: c_ssize_t =
        unsafe { write(STDOUT_FD, (&raw const byte).cast::<c_void>(), 1 as c_size_t) };
    if ret < 0 {
        -1
    } else {
        byte as c_int
    }
}
