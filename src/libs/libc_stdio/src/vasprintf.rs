// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::VaList;
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
/// Formats output according to `fmt` and the argument list `ap`, allocating a buffer large enough
/// to hold the result (including the terminating null byte) with `malloc()`. On success the address
/// of the allocated buffer is stored in `*strp`; the caller is responsible for freeing it.
///
/// # Parameters
///
/// - `strp`: Output pointer that receives the address of the allocated, formatted string.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `ap`: Variable argument list matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written (excluding the terminating null byte) on success, or `-1` on
/// error. On error the contents of `*strp` are undefined and no memory is left allocated.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `strp` points to a writable `char *` location.
/// - `fmt` points to a valid, null-terminated format string.
/// - `ap` provides arguments matching the format specifiers.
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man3/asprintf.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vasprintf(
    strp: *mut *mut c_char,
    fmt: *const c_char,
    ap: VaList<'_>,
) -> c_int {
    extern "C" {
        fn malloc(size: c_size_t) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    if strp.is_null() {
        return -1;
    }

    // Measure the formatted length using an independent cursor over the argument list.
    let measure: VaList<'_> = ap.clone();
    // SAFETY: a null buffer of size zero is valid for vsnprintf; it only computes the length.
    let len: c_int = unsafe { crate::vsnprintf::vsnprintf(core::ptr::null_mut(), 0, fmt, measure) };
    if len < 0 {
        return -1;
    }

    let size: c_size_t = (len as c_size_t) + 1;
    // SAFETY: allocating the measured number of bytes plus the null terminator.
    let buf: *mut c_void = unsafe { malloc(size) };
    if buf.is_null() {
        return -1;
    }

    let out: *mut c_char = buf.cast::<c_char>();
    // SAFETY: out points to `size` writable bytes; ap matches the format specifiers.
    let ret: c_int = unsafe { crate::vsnprintf::vsnprintf(out, size, fmt, ap) };
    if ret < 0 {
        // SAFETY: buf was returned by malloc and has not been freed.
        unsafe { free(buf) };
        return -1;
    }

    *strp = out;
    ret
}
