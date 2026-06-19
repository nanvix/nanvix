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
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Calculates the length of a string, examining at most `maxlen` bytes.
///
/// This function computes the length of the string pointed to by `s`, excluding the terminating
/// null byte (`'\0'`), but examines at most `maxlen` bytes.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string whose length is to be calculated.
/// - `maxlen`: Maximum number of bytes to examine.
///
/// # Return Value
///
/// Returns the number of characters in the string pointed to by `s`, excluding the terminating
/// null byte, or `maxlen` if no null byte is found within that many bytes.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strnlen(s: *const c_char, maxlen: c_size_t) -> c_size_t {
    debug_assert!(!s.is_null(), "strnlen(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strnlen(): pointer is not properly aligned"
    );

    let mut i: c_size_t = 0;
    while i < maxlen && *s.add(i as usize) != 0 {
        i += 1;
    }
    i
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strnlen;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_char;

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0 as c_char);
        v
    }

    #[test]
    fn test_strnlen_shorter_than_maxlen() {
        let buf: Vec<c_char> = make_c_string(b"hello");
        let len: usize = unsafe { strnlen(buf.as_ptr(), 100) as usize };
        assert_eq!(len, 5, "strnlen should return actual length when shorter than maxlen");
    }

    #[test]
    fn test_strnlen_longer_than_maxlen() {
        let buf: Vec<c_char> = make_c_string(b"hello world");
        let len: usize = unsafe { strnlen(buf.as_ptr(), 5) as usize };
        assert_eq!(len, 5, "strnlen should return maxlen when string is longer");
    }

    #[test]
    fn test_strnlen_empty_string() {
        let buf: Vec<c_char> = make_c_string(b"");
        let len: usize = unsafe { strnlen(buf.as_ptr(), 100) as usize };
        assert_eq!(len, 0, "strnlen of empty string should be 0");
    }

    #[test]
    fn test_strnlen_zero_maxlen() {
        let buf: Vec<c_char> = make_c_string(b"hello");
        let len: usize = unsafe { strnlen(buf.as_ptr(), 0) as usize };
        assert_eq!(len, 0, "strnlen with maxlen 0 should return 0");
    }

    #[test]
    fn test_strnlen_exact_maxlen() {
        let buf: Vec<c_char> = make_c_string(b"hello");
        let len: usize = unsafe { strnlen(buf.as_ptr(), 5) as usize };
        assert_eq!(len, 5, "strnlen should return 5 when maxlen equals string length");
    }
}
