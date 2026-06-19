// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Locates the first occurrence of the wide substring `needle` in the wide string `haystack`.
///
/// # Safety
///
/// `haystack` and `needle` must point to valid, null-terminated wide strings.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsstr(haystack: *const wchar_t, needle: *const wchar_t) -> *mut wchar_t {
    if unsafe { *needle } == 0 {
        return haystack.cast_mut();
    }

    let mut h: *const wchar_t = haystack;
    while unsafe { *h } != 0 {
        let mut a: *const wchar_t = h;
        let mut b: *const wchar_t = needle;
        while unsafe { *b } != 0 && unsafe { *a } == unsafe { *b } {
            a = unsafe { a.add(1) };
            b = unsafe { b.add(1) };
        }
        if unsafe { *b } == 0 {
            return h.cast_mut();
        }
        h = unsafe { h.add(1) };
    }
    core::ptr::null_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcsstr_found() {
        // haystack = "hello", needle = "ll".
        let haystack: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
        let needle: [wchar_t; 3] = [0x6C, 0x6C, 0];
        let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
        assert_eq!(result.cast_const(), unsafe { haystack.as_ptr().add(2) });
    }

    #[test]
    fn test_wcsstr_not_found() {
        let haystack: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let needle: [wchar_t; 2] = [0x64, 0];
        let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_wcsstr_empty_needle() {
        // An empty needle matches at the start of the haystack.
        let haystack: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let needle: [wchar_t; 1] = [0];
        let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
        assert_eq!(result.cast_const(), haystack.as_ptr());
    }

    #[test]
    fn test_wcsstr_whole_string() {
        let haystack: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let needle: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
        assert_eq!(result.cast_const(), haystack.as_ptr());
    }
}
