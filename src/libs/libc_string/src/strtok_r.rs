// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Checks whether `c` is one of the delimiter characters in `delim`.
unsafe fn is_delim(c: c_char, delim: *const c_char) -> bool {
    let mut d: usize = 0;
    while *delim.add(d) != 0 {
        if c == *delim.add(d) {
            return true;
        }
        d += 1;
    }
    false
}

///
/// # Description
///
/// Extracts tokens from strings (reentrant version).
///
/// This function breaks a string into a sequence of zero or more non-empty tokens. On the first
/// call, `s` should point to the string to be parsed. On subsequent calls to obtain subsequent
/// tokens, `s` should be null. The `saveptr` argument is used to maintain context between
/// successive calls that parse the same string.
///
/// # Parameters
///
/// - `s`: Pointer to the string to tokenize, or null for subsequent calls.
/// - `delim`: Pointer to a null-terminated string of delimiter characters.
/// - `saveptr`: Pointer to a `*mut c_char` variable used to maintain context.
///
/// # Return Value
///
/// Returns a pointer to the next token, or a null pointer if there are no more tokens.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes null bytes into the original string to terminate tokens.
/// - It writes the continuation pointer through `saveptr`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtok_r(
    s: *mut c_char,
    delim: *const c_char,
    saveptr: *mut *mut c_char,
) -> *mut c_char {
    debug_assert!(!delim.is_null(), "strtok_r(): null delimiter pointer");
    debug_assert!(!saveptr.is_null(), "strtok_r(): null saveptr");
    debug_assert!(
        (delim as usize).is_multiple_of(align_of::<c_char>()),
        "strtok_r(): delimiter pointer is not properly aligned"
    );

    let mut current: *mut c_char = if !s.is_null() { s } else { *saveptr };

    if current.is_null() {
        return core::ptr::null_mut();
    }

    // Skip leading delimiters.
    while *current != 0 && is_delim(*current, delim) {
        current = current.add(1);
    }

    if *current == 0 {
        *saveptr = core::ptr::null_mut();
        return core::ptr::null_mut();
    }

    let token: *mut c_char = current;

    // Find end of token.
    while *current != 0 && !is_delim(*current, delim) {
        current = current.add(1);
    }

    if *current != 0 {
        *current = 0 as c_char;
        *saveptr = current.add(1);
    } else {
        *saveptr = core::ptr::null_mut();
    }

    token
}

#[cfg(all(test, feature = "std"))]
mod test;
