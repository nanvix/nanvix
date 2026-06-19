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
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_uchar,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares at most `n` bytes of two null-terminated strings.
///
/// This function compares the strings pointed to by `s1` and `s2` byte by byte, interpreting each
/// byte as an unsigned char (`c_uchar`). The comparison stops at the first differing byte, when a
/// null terminator is reached, or after `n` bytes have been compared.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated string.
/// - `s2`: Pointer to the second null-terminated string.
/// - `n`: Maximum number of bytes to compare.
///
/// # Return Value
///
/// Returns an integer less than, equal to, or greater than zero if the first `n` bytes of `s1` are
/// found, respectively, to be less than, to match, or be greater than the first `n` bytes of `s2`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s1` and `s2` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strncmp(s1: *const c_char, s2: *const c_char, n: c_size_t) -> c_int {
    debug_assert!(!s1.is_null(), "strncmp(): null pointer");
    debug_assert!(!s2.is_null(), "strncmp(): null pointer");
    debug_assert!(
        (s1 as usize).is_multiple_of(align_of::<c_char>()),
        "strncmp(): pointer is not properly aligned"
    );
    debug_assert!(
        (s2 as usize).is_multiple_of(align_of::<c_char>()),
        "strncmp(): pointer is not properly aligned"
    );

    let a: *const c_uchar = s1.cast::<c_uchar>();
    let b: *const c_uchar = s2.cast::<c_uchar>();
    let n: usize = n as usize;
    let mut i: usize = 0;
    while i < n {
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
    0
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strncmp;
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
    fn test_strncmp_equal_within_n() {
        let s1: Vec<c_char> = make_c_string(b"hello");
        let s2: Vec<c_char> = make_c_string(b"hello");
        let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 5) };
        assert_eq!(ret, 0, "strncmp should return 0 for equal strings within n");
    }

    #[test]
    fn test_strncmp_differ_within_n() {
        let s1: Vec<c_char> = make_c_string(b"helly");
        let s2: Vec<c_char> = make_c_string(b"hello");
        let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 5) };
        assert!(ret != 0, "strncmp should return non-zero when strings differ within n");
    }

    #[test]
    fn test_strncmp_n_zero() {
        let s1: Vec<c_char> = make_c_string(b"abc");
        let s2: Vec<c_char> = make_c_string(b"xyz");
        let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 0) };
        assert_eq!(ret, 0, "strncmp should return 0 when n is 0");
    }

    #[test]
    fn test_strncmp_n_greater_than_length() {
        let s1: Vec<c_char> = make_c_string(b"ab");
        let s2: Vec<c_char> = make_c_string(b"ab");
        let ret: c_int = unsafe { strncmp(s1.as_ptr(), s2.as_ptr(), 100) };
        assert_eq!(ret, 0, "strncmp should return 0 for equal strings even when n > length");
    }
}
