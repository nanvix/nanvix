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
mod test {
    use super::strncasecmp;
    use ::std::vec::Vec;
    use ::sysapi::ffi::{
        c_char,
        c_int,
    };

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0 as c_char);
        v
    }

    #[test]
    fn test_strncasecmp_equal_within_n() {
        let s1: Vec<c_char> = make_c_string(b"Hello");
        let s2: Vec<c_char> = make_c_string(b"hELLO");
        let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 5) };
        assert_eq!(ret, 0, "strncasecmp should ignore case within n bytes");
    }

    #[test]
    fn test_strncasecmp_n_limits_comparison() {
        // Strings differ only after the first three bytes; n=3 ignores the rest.
        let s1: Vec<c_char> = make_c_string(b"abcXXX");
        let s2: Vec<c_char> = make_c_string(b"abcYYY");
        let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 3) };
        assert_eq!(ret, 0, "strncasecmp should only compare the first n bytes");
    }

    #[test]
    fn test_strncasecmp_differ_within_n() {
        let s1: Vec<c_char> = make_c_string(b"abc");
        let s2: Vec<c_char> = make_c_string(b"abz");
        let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 3) };
        assert!(ret < 0, "strncasecmp should return negative when s1 < s2 within n");
    }

    #[test]
    fn test_strncasecmp_zero_n() {
        let s1: Vec<c_char> = make_c_string(b"abc");
        let s2: Vec<c_char> = make_c_string(b"xyz");
        let ret: c_int = unsafe { strncasecmp(s1.as_ptr(), s2.as_ptr(), 0) };
        assert_eq!(ret, 0, "strncasecmp should return 0 when n is 0");
    }
}
