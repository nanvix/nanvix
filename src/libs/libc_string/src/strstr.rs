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
/// Locates a substring.
///
/// This function finds the first occurrence of the substring `needle` in the string `haystack`.
/// The terminating null bytes are not compared. If `needle` is an empty string, `haystack` is
/// returned.
///
/// # Parameters
///
/// - `haystack`: Pointer to the null-terminated string to search in.
/// - `needle`: Pointer to the null-terminated substring to search for.
///
/// # Return Value
///
/// Returns a pointer to the beginning of the located substring, or a null pointer if the substring
/// is not found.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `haystack` and `needle` without bounds
///   checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    debug_assert!(!haystack.is_null(), "strstr(): null haystack pointer");
    debug_assert!(!needle.is_null(), "strstr(): null needle pointer");
    debug_assert!(
        (haystack as usize).is_multiple_of(align_of::<c_char>()),
        "strstr(): haystack pointer is not properly aligned"
    );
    debug_assert!(
        (needle as usize).is_multiple_of(align_of::<c_char>()),
        "strstr(): needle pointer is not properly aligned"
    );

    // Empty needle: return haystack.
    if *needle == 0 {
        return haystack.cast_mut();
    }

    let mut h: usize = 0;
    while *haystack.add(h) != 0 {
        let mut hi: usize = h;
        let mut ni: usize = 0;
        while *needle.add(ni) != 0 && *haystack.add(hi) == *needle.add(ni) {
            hi += 1;
            ni += 1;
        }
        if *needle.add(ni) == 0 {
            return haystack.add(h).cast_mut();
        }
        h += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strstr;
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
    fn test_strstr_found() {
        let haystack: Vec<c_char> = make_c_string(b"hello world");
        let needle: Vec<c_char> = make_c_string(b"world");
        let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(!ret.is_null(), "strstr should find 'world'");
        let offset: usize = usize::try_from(unsafe { ret.offset_from(haystack.as_ptr()) })
            .expect("offset is non-negative");
        assert_eq!(offset, 6, "'world' starts at index 6");
    }

    #[test]
    fn test_strstr_not_found() {
        let haystack: Vec<c_char> = make_c_string(b"hello world");
        let needle: Vec<c_char> = make_c_string(b"xyz");
        let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(ret.is_null(), "strstr should return null when needle not found");
    }

    #[test]
    fn test_strstr_empty_needle() {
        let haystack: Vec<c_char> = make_c_string(b"hello");
        let needle: Vec<c_char> = make_c_string(b"");
        let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
        assert_eq!(ret, haystack.as_ptr().cast_mut(), "empty needle returns haystack");
    }

    #[test]
    fn test_strstr_at_start() {
        let haystack: Vec<c_char> = make_c_string(b"hello");
        let needle: Vec<c_char> = make_c_string(b"hell");
        let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(!ret.is_null(), "strstr should find needle at start");
        let offset: usize = usize::try_from(unsafe { ret.offset_from(haystack.as_ptr()) })
            .expect("offset is non-negative");
        assert_eq!(offset, 0, "needle at start should be at index 0");
    }

    #[test]
    fn test_strstr_at_end() {
        let haystack: Vec<c_char> = make_c_string(b"hello");
        let needle: Vec<c_char> = make_c_string(b"llo");
        let ret: *mut c_char = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(!ret.is_null(), "strstr should find needle at end");
        let offset: usize = usize::try_from(unsafe { ret.offset_from(haystack.as_ptr()) })
            .expect("offset is non-negative");
        assert_eq!(offset, 2, "'llo' starts at index 2");
    }
}
