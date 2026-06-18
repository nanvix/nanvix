// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(non_camel_case_types)]

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
// Types and Constants
//==================================================================================================

type wchar_t = i32;

/// Value returned by the multibyte conversion functions on an encoding error (`(size_t)-1`).
const SIZE_ERR: c_size_t = c_size_t::MAX;

//==================================================================================================
// External Symbols
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn wcslen(s: *const wchar_t) -> c_size_t;
    fn wcstombs(dst: *mut c_char, src: *const wchar_t, n: c_size_t) -> c_size_t;
    fn mbstowcs(dst: *mut wchar_t, src: *const c_char, n: c_size_t) -> c_size_t;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Writes formatted output to the wide-character buffer `ws` (at most `n` wide characters including
/// the terminator), taking arguments from `ap`.
///
/// # Description
///
/// The wide format string is converted to UTF-8 and rendered with the narrow `vsnprintf()` engine;
/// the resulting bytes are converted back to wide characters. This reuses the single, well-tested
/// formatting implementation for both the narrow and wide entry points.
///
/// # Safety
///
/// `ws` must have room for `n` wide characters and `format` must be a valid wide format string
/// matching the variadic arguments in `ap`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vswprintf(
    ws: *mut wchar_t,
    n: c_size_t,
    format: *const wchar_t,
    ap: VaList<'_>,
) -> c_int {
    if n == 0 {
        return -1;
    }

    // Reject null buffers before dereferencing them.
    if ws.is_null() || format.is_null() {
        return -1;
    }

    // Convert the wide format string to narrow UTF-8.
    let flen: c_size_t = unsafe { wcslen(format) };
    let nfmt_size: c_size_t = flen.saturating_mul(4).saturating_add(1);
    let nfmt: *mut c_char = unsafe { malloc(nfmt_size) }.cast::<c_char>();
    if nfmt.is_null() {
        return -1;
    }
    // On an encoding error wcstombs() returns (size_t)-1; fail and free the buffer.
    if unsafe { wcstombs(nfmt, format, nfmt_size) } == SIZE_ERR {
        unsafe { free(nfmt.cast::<c_void>()) };
        return -1;
    }

    // Render into a narrow buffer large enough to back `n` wide characters.
    let nout_size: c_size_t = n.saturating_mul(4).saturating_add(4);
    let nout: *mut c_char = unsafe { malloc(nout_size) }.cast::<c_char>();
    if nout.is_null() {
        unsafe { free(nfmt.cast::<c_void>()) };
        return -1;
    }
    unsafe { crate::vsnprintf::vsnprintf(nout, nout_size, nfmt, ap) };

    // Convert the formatted narrow output back to wide characters.
    let max_chars: c_size_t = n - 1;
    let res: c_size_t = unsafe { mbstowcs(ws, nout, max_chars) };
    unsafe {
        free(nfmt.cast::<c_void>());
        free(nout.cast::<c_void>());
    }
    if res == SIZE_ERR {
        return -1;
    }

    let written: c_size_t = if res > max_chars { max_chars } else { res };
    unsafe { *ws.add(written as usize) = 0 };
    written as c_int
}

/// Writes formatted output to the wide-character buffer `ws` (at most `n` wide characters including
/// the terminator).
///
/// # Safety
///
/// See [`vswprintf`].
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn swprintf(
    ws: *mut wchar_t,
    n: c_size_t,
    format: *const wchar_t,
    args: ...
) -> c_int {
    unsafe { vswprintf(ws, n, format, args) }
}
