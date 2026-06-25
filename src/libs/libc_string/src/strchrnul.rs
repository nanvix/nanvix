// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// Locates the first occurrence of a character in a string, never returning a null pointer.
///
/// This function behaves like `strchr()`: it returns a pointer to the first occurrence of the
/// character `c` (converted to a `c_char`) in the string pointed to by `s`. Unlike `strchr()`,
/// however, if `c` is not found it returns a pointer to the terminating null byte of `s` rather
/// than a null pointer. It is a GNU extension.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `c`: Character to locate (converted to `c_char`).
///
/// # Return Value
///
/// Returns a pointer to the located character, or a pointer to the terminating null byte of `s` if
/// the character does not occur in the string.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
/// It is safe to call this function if and only if `s` points to a valid, null-terminated string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strchrnul(s: *const c_char, c: c_int) -> *mut c_char {
    debug_assert!(!s.is_null(), "strchrnul(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strchrnul(): pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let p: *const c_uchar = s.cast::<c_uchar>();
    let mut i: usize = 0;
    loop {
        let ch: c_uchar = *p.add(i);
        if ch == target || ch == 0 {
            return s.add(i).cast_mut();
        }
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strchrnul;
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
    fn test_strchrnul_found() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), b'l' as c_int) };
        let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
        assert_eq!(offset, 2, "strchrnul should find the first 'l' at index 2");
    }

    #[test]
    fn test_strchrnul_not_found_returns_terminator() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), b'z' as c_int) };
        let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
        assert_eq!(offset, 5, "strchrnul should return the terminator when char is not found");
        assert_eq!(unsafe { *ret }, 0 as c_char, "the returned pointer must point at a null byte");
    }

    #[test]
    fn test_strchrnul_find_null_terminator() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), 0) };
        let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
        assert_eq!(offset, 5, "strchrnul(.., 0) should return the terminator");
    }

    #[test]
    fn test_strchrnul_empty_string() {
        let s: Vec<c_char> = make_c_string(b"");
        let ret: *mut c_char = unsafe { strchrnul(s.as_ptr(), b'a' as c_int) };
        let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
        assert_eq!(offset, 0, "strchrnul of an empty string should return the start (terminator)");
    }
}
