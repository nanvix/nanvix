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
/// Locates the last occurrence of a character in a string.
///
/// This function returns a pointer to the last occurrence of the character `c` (converted to a
/// `c_char`) in the string pointed to by `s`. The terminating null byte is considered part of the
/// string.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `c`: Character to locate (converted to `c_char`).
///
/// # Return Value
///
/// Returns a pointer to the last occurrence of the character, or a null pointer if the character
/// does not occur in the string.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strrchr(s: *const c_char, c: c_int) -> *mut c_char {
    debug_assert!(!s.is_null(), "strrchr(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strrchr(): pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let p: *const c_uchar = s.cast::<c_uchar>();
    let mut last: *mut c_char = core::ptr::null_mut();
    let mut i: usize = 0;
    loop {
        let ch: c_uchar = *p.add(i);
        if ch == target {
            last = s.add(i).cast_mut();
        }
        if ch == 0 {
            return last;
        }
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strrchr;
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
    fn test_strrchr_multiple_occurrences() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let ret: *mut c_char = unsafe { strrchr(s.as_ptr(), b'l' as c_int) };
        assert!(!ret.is_null(), "strrchr should find 'l'");
        let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
        assert_eq!(offset, 3, "last 'l' should be at index 3");
    }

    #[test]
    fn test_strrchr_single_occurrence() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let ret: *mut c_char = unsafe { strrchr(s.as_ptr(), b'h' as c_int) };
        assert!(!ret.is_null(), "strrchr should find 'h'");
        let offset: usize = unsafe { ret.offset_from(s.as_ptr()) } as usize;
        assert_eq!(offset, 0, "'h' should be at index 0");
    }

    #[test]
    fn test_strrchr_not_found() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let ret: *mut c_char = unsafe { strrchr(s.as_ptr(), b'z' as c_int) };
        assert!(ret.is_null(), "strrchr should return null for char not in string");
    }
}
