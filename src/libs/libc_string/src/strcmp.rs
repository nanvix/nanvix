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
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_uchar,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares two null-terminated strings.
///
/// This function compares the strings pointed to by `s1` and `s2` byte by byte, interpreting each
/// byte as an unsigned char (`c_uchar`). The comparison stops at the first differing byte or when
/// a null terminator is reached.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated string.
/// - `s2`: Pointer to the second null-terminated string.
///
/// # Return Value
///
/// Returns an integer less than, equal to, or greater than zero if `s1` is found, respectively, to
/// be less than, to match, or be greater than `s2`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s1` and `s2` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
    debug_assert!(!s1.is_null(), "strcmp(): null pointer");
    debug_assert!(!s2.is_null(), "strcmp(): null pointer");
    debug_assert!(
        (s1 as usize).is_multiple_of(align_of::<c_char>()),
        "strcmp(): pointer is not properly aligned"
    );
    debug_assert!(
        (s2 as usize).is_multiple_of(align_of::<c_char>()),
        "strcmp(): pointer is not properly aligned"
    );

    let a: *const c_uchar = s1.cast::<c_uchar>();
    let b: *const c_uchar = s2.cast::<c_uchar>();
    let mut i: usize = 0;
    loop {
        let va: c_uchar = *a.add(i);
        let vb: c_uchar = *b.add(i);
        if va != vb {
            return (va as c_int) - (vb as c_int);
        }
        if va == 0 {
            return 0;
        }
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strcmp;
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
    fn test_strcmp_equal() {
        let s1: Vec<c_char> = make_c_string(b"hello");
        let s2: Vec<c_char> = make_c_string(b"hello");
        let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
        assert_eq!(ret, 0, "strcmp should return 0 for equal strings");
    }

    #[test]
    fn test_strcmp_first_less() {
        let s1: Vec<c_char> = make_c_string(b"abc");
        let s2: Vec<c_char> = make_c_string(b"abz");
        let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
        assert!(ret < 0, "strcmp should return negative when s1 < s2");
    }

    #[test]
    fn test_strcmp_first_greater() {
        let s1: Vec<c_char> = make_c_string(b"abz");
        let s2: Vec<c_char> = make_c_string(b"abc");
        let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
        assert!(ret > 0, "strcmp should return positive when s1 > s2");
    }

    #[test]
    fn test_strcmp_empty_strings() {
        let s1: Vec<c_char> = make_c_string(b"");
        let s2: Vec<c_char> = make_c_string(b"");
        let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
        assert_eq!(ret, 0, "strcmp should return 0 for two empty strings");
    }

    #[test]
    fn test_strcmp_one_empty() {
        let s1: Vec<c_char> = make_c_string(b"");
        let s2: Vec<c_char> = make_c_string(b"a");
        let ret: c_int = unsafe { strcmp(s1.as_ptr(), s2.as_ptr()) };
        assert!(ret < 0, "strcmp should return negative when s1 is empty and s2 is not");
    }
}
