// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::ffi::c_char;

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
/// Locates the first case-insensitive occurrence of a substring in a string.
///
/// This function behaves like `strstr()` but ignores the case of ASCII letters when matching: it
/// returns a pointer to the first occurrence within the string pointed to by `haystack` of the
/// sequence of characters in the string pointed to by `needle`, comparing letters without regard
/// to case. It is a GNU extension.
///
/// # Parameters
///
/// - `haystack`: Pointer to the null-terminated string to be searched.
/// - `needle`: Pointer to the null-terminated substring to search for.
///
/// # Return Value
///
/// Returns a pointer to the beginning of the located substring, or a null pointer if the substring
/// is not found. If `needle` points to an empty string, `haystack` is returned.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `haystack` and `needle` without bounds
///   checking.
///
/// It is safe to call this function if and only if both `haystack` and `needle` point to valid,
/// null-terminated strings.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcasestr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    debug_assert!(!haystack.is_null(), "strcasestr(): null haystack pointer");
    debug_assert!(!needle.is_null(), "strcasestr(): null needle pointer");
    debug_assert!(
        (haystack as usize).is_multiple_of(align_of::<c_char>()),
        "strcasestr(): haystack pointer is not properly aligned"
    );
    debug_assert!(
        (needle as usize).is_multiple_of(align_of::<c_char>()),
        "strcasestr(): needle pointer is not properly aligned"
    );

    // Empty needle: return haystack.
    if *needle == 0 {
        return haystack.cast_mut();
    }

    let mut h: usize = 0;
    while *haystack.add(h) != 0 {
        let mut hi: usize = h;
        let mut ni: usize = 0;
        while *needle.add(ni) != 0
            && to_lower(*haystack.add(hi) as u8) == to_lower(*needle.add(ni) as u8)
        {
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
    use super::strcasestr;
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
    fn test_strcasestr_found_case_insensitive() {
        let haystack: Vec<c_char> = make_c_string(b"Hello, World");
        let needle: Vec<c_char> = make_c_string(b"world");
        let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(!ret.is_null(), "strcasestr should find 'world' ignoring case");
        let offset: usize = unsafe { ret.offset_from(haystack.as_ptr()) } as usize;
        assert_eq!(offset, 7, "match should start at index 7");
    }

    #[test]
    fn test_strcasestr_not_found() {
        let haystack: Vec<c_char> = make_c_string(b"Hello");
        let needle: Vec<c_char> = make_c_string(b"xyz");
        let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(ret.is_null(), "strcasestr should return null when the needle is not found");
    }

    #[test]
    fn test_strcasestr_empty_needle() {
        let haystack: Vec<c_char> = make_c_string(b"Hello");
        let needle: Vec<c_char> = make_c_string(b"");
        let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
        assert_eq!(ret, haystack.as_ptr().cast_mut(), "empty needle should return haystack");
    }

    #[test]
    fn test_strcasestr_prefix_match() {
        let haystack: Vec<c_char> = make_c_string(b"ABCabc");
        let needle: Vec<c_char> = make_c_string(b"abc");
        let ret: *mut c_char = unsafe { strcasestr(haystack.as_ptr(), needle.as_ptr()) };
        let offset: usize = unsafe { ret.offset_from(haystack.as_ptr()) } as usize;
        assert_eq!(offset, 0, "strcasestr should match the first (case-folded) occurrence");
    }
}
