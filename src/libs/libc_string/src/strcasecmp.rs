// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
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
/// Compares the strings `s1` and `s2` ignoring the case of ASCII letters.
///
/// # Returns
///
/// A negative, zero, or positive value if `s1` is respectively less than, equal to, or greater
/// than `s2` after case folding.
///
/// # Safety
///
/// Both `s1` and `s2` must point to valid, null-terminated strings.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/strcasecmp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int {
    debug_assert!(!s1.is_null(), "strcasecmp(): null pointer");
    debug_assert!(!s2.is_null(), "strcasecmp(): null pointer");

    let mut a: *const c_char = s1;
    let mut b: *const c_char = s2;
    loop {
        let ca: u8 = to_lower(*a as u8);
        let cb: u8 = to_lower(*b as u8);
        if ca != cb {
            return c_int::from(ca) - c_int::from(cb);
        }
        if ca == 0 {
            return 0;
        }
        a = a.add(1);
        b = b.add(1);
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strcasecmp;
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
    fn test_strcasecmp_equal_ignoring_case() {
        let s1: Vec<c_char> = make_c_string(b"Hello");
        let s2: Vec<c_char> = make_c_string(b"hELLO");
        let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
        assert_eq!(ret, 0, "strcasecmp should ignore case for equal strings");
    }

    #[test]
    fn test_strcasecmp_less() {
        let s1: Vec<c_char> = make_c_string(b"abc");
        let s2: Vec<c_char> = make_c_string(b"abz");
        let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
        assert!(ret < 0, "strcasecmp should return negative when s1 < s2");
    }

    #[test]
    fn test_strcasecmp_greater_case_insensitive() {
        // Case-folded 'Z' (0x7A) is greater than 'c' (0x63).
        let s1: Vec<c_char> = make_c_string(b"abZ");
        let s2: Vec<c_char> = make_c_string(b"abc");
        let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
        assert!(ret > 0, "strcasecmp should compare case-insensitively");
    }

    #[test]
    fn test_strcasecmp_empty_strings() {
        let s1: Vec<c_char> = make_c_string(b"");
        let s2: Vec<c_char> = make_c_string(b"");
        let ret: c_int = unsafe { strcasecmp(s1.as_ptr(), s2.as_ptr()) };
        assert_eq!(ret, 0, "two empty strings should be equal");
    }
}
