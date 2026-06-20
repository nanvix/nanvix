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

///
/// # Description
///
/// Searches a string for any of a set of bytes.
///
/// This function locates the first occurrence in the string `s` of any byte in the string `accept`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `accept`: Pointer to a null-terminated string of bytes to search for.
///
/// # Return Value
///
/// Returns a pointer to the byte in `s` that matches one of the bytes in `accept`, or a null
/// pointer if no such byte is found.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s` and `accept` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strpbrk(s: *const c_char, accept: *const c_char) -> *mut c_char {
    debug_assert!(!s.is_null(), "strpbrk(): null pointer");
    debug_assert!(!accept.is_null(), "strpbrk(): null accept pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strpbrk(): pointer is not properly aligned"
    );
    debug_assert!(
        (accept as usize).is_multiple_of(align_of::<c_char>()),
        "strpbrk(): accept pointer is not properly aligned"
    );

    let mut i: usize = 0;
    while *s.add(i) != 0 {
        let ch: c_char = *s.add(i);
        let mut a: usize = 0;
        while *accept.add(a) != 0 {
            if ch == *accept.add(a) {
                return s.add(i).cast_mut();
            }
            a += 1;
        }
        i += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strpbrk;
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
    fn test_strpbrk_found() {
        let s: Vec<c_char> = make_c_string(b"hello world");
        let accept: Vec<c_char> = make_c_string(b"ow");
        let ret: *mut c_char = unsafe { strpbrk(s.as_ptr(), accept.as_ptr()) };
        assert!(!ret.is_null(), "strpbrk should find a match");
        let offset: usize = usize::try_from(unsafe { ret.offset_from(s.as_ptr()) })
            .expect("offset is non-negative");
        assert_eq!(offset, 4, "first 'o' is at index 4");
    }

    #[test]
    fn test_strpbrk_not_found() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let accept: Vec<c_char> = make_c_string(b"xyz");
        let ret: *mut c_char = unsafe { strpbrk(s.as_ptr(), accept.as_ptr()) };
        assert!(ret.is_null(), "strpbrk should return null when no match");
    }

    #[test]
    fn test_strpbrk_empty_accept() {
        let s: Vec<c_char> = make_c_string(b"hello");
        let accept: Vec<c_char> = make_c_string(b"");
        let ret: *mut c_char = unsafe { strpbrk(s.as_ptr(), accept.as_ptr()) };
        assert!(ret.is_null(), "strpbrk should return null with empty accept");
    }
}
