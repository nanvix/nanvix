// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    errno::{
        EINVAL,
        ERANGE,
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stores a textual description of the error number `errnum` in `buf`.
///
/// # Parameters
///
/// - `errnum`: Error number to describe.
/// - `buf`: Buffer that receives the message.
/// - `buflen`: Size of `buf` in bytes.
///
/// # Returns
///
/// Returns zero on success. Otherwise, returns an error number to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. `buf` must point to at least
/// `buflen` writable bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/strerror_r.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: c_size_t) -> c_int {
    if buf.is_null() {
        return EINVAL;
    }

    // SAFETY: strerror returns a valid, null-terminated static string.
    let src: *const c_char = unsafe { crate::strerror::strerror(errnum) };

    // SAFETY: src is a valid, null-terminated string.
    let len: usize = unsafe { crate::strlen::strlen(src) } as usize;
    if buflen == 0 {
        return ERANGE;
    }

    let cap: usize = buflen as usize - 1;
    let copy_len: usize = if len < cap { len } else { cap };
    // SAFETY: src has at least copy_len bytes; buf has room for copy_len + 1.
    unsafe {
        core::ptr::copy_nonoverlapping(src.cast::<u8>(), buf.cast::<u8>(), copy_len);
        *buf.add(copy_len) = 0;
    }

    if len >= buflen as usize {
        ERANGE
    } else {
        0
    }
}

#[cfg(all(test, feature = "std"))]
mod test;
