// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![cfg_attr(not(feature = "std"), no_std)]
// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

pub mod basename;
pub mod dirname;

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_uchar,
};

//==================================================================================================
// Internal Helpers
//==================================================================================================

/// Returns the number of bytes in the null-terminated C string `p`, excluding the terminator.
///
/// # Safety
///
/// `p` must point to a valid null-terminated C string.
pub(crate) unsafe fn c_len(p: *const c_uchar) -> usize {
    let mut n: usize = 0;
    while *p.add(n) != 0 {
        n += 1;
    }
    n
}

/// Returns a pointer to the static `"."` string used when a path has no directory component.
///
/// The returned buffer must be treated as read-only by callers, matching the POSIX contract that
/// the result may reference statically allocated storage.
pub(crate) fn dot_ptr() -> *mut c_char {
    static DOT: [u8; 2] = [b'.', 0];
    (&raw const DOT).cast::<c_char>().cast_mut()
}

/// Returns a pointer to the static `"/"` string used when a path's directory component is the root.
///
/// The returned buffer must be treated as read-only by callers, matching the POSIX contract that
/// the result may reference statically allocated storage.
pub(crate) fn slash_ptr() -> *mut c_char {
    static SLASH: [u8; 2] = [b'/', 0];
    (&raw const SLASH).cast::<c_char>().cast_mut()
}
