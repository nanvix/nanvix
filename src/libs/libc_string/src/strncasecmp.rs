// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Helpers
//==================================================================================================

/// Folds an ASCII upper-case byte to lower case.
#[inline]
fn to_lower(b: u8) -> u8 {
    if b.is_ascii_uppercase() {
        b + 32
    } else {
        b
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares at most `n` bytes of the strings `s1` and `s2` ignoring the case of ASCII letters.
///
/// # Returns
///
/// A negative, zero, or positive value if `s1` is respectively less than, equal to, or greater
/// than `s2` after case folding, considering at most `n` bytes.
///
/// # Safety
///
/// Both `s1` and `s2` must point to valid, null-terminated strings.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/strncasecmp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strncasecmp(s1: *const c_char, s2: *const c_char, n: c_size_t) -> c_int {
    debug_assert!(!s1.is_null(), "strncasecmp(): null pointer");
    debug_assert!(!s2.is_null(), "strncasecmp(): null pointer");

    let mut i: c_size_t = 0;
    while i < n {
        let ca: u8 = to_lower(*s1.add(i as usize) as u8);
        let cb: u8 = to_lower(*s2.add(i as usize) as u8);
        if ca != cb {
            return c_int::from(ca) - c_int::from(cb);
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

#[cfg(all(test, feature = "std"))]
mod test;
