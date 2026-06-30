// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Locates the first occurrence in the wide string `ws1` of any wide character from the wide string
/// `ws2`.
///
/// # Parameters
///
/// - `ws1`: Pointer to the null-terminated wide string to search.
/// - `ws2`: Pointer to a null-terminated wide string containing the set of wide characters to
///   match.
///
/// # Return Value
///
/// Returns a pointer to the first wide character in `ws1` that matches any wide character in
/// `ws2`, or a null pointer if no such wide character is found.
///
/// # Safety
///
/// Behavior is undefined if `ws1` or `ws2` is null or does not point to a valid null-terminated
/// wide string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcspbrk(ws1: *const wchar_t, ws2: *const wchar_t) -> *mut wchar_t {
    debug_assert!(!ws1.is_null(), "wcspbrk(): null ws1 pointer");
    debug_assert!(!ws2.is_null(), "wcspbrk(): null ws2 pointer");

    let mut s: *const wchar_t = ws1;
    while unsafe { *s } != 0 {
        let ch: wchar_t = unsafe { *s };
        let mut a: *const wchar_t = ws2;
        while unsafe { *a } != 0 {
            if ch == unsafe { *a } {
                return s.cast_mut();
            }
            a = unsafe { a.add(1) };
        }
        s = unsafe { s.add(1) };
    }

    ::core::ptr::null_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcspbrk_found() {
        // ws1 = "hello world", ws2 = "ow" -> first match is 'o' at index 4.
        let ws1: [wchar_t; 12] = [
            0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x77, 0x6F, 0x72, 0x6C, 0x64, 0,
        ];
        let ws2: [wchar_t; 3] = [0x6F, 0x77, 0];
        let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
        assert_eq!(result.cast_const(), unsafe { ws1.as_ptr().add(4) });
    }

    #[test]
    fn test_wcspbrk_not_found() {
        let ws1: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
        let ws2: [wchar_t; 4] = [0x78, 0x79, 0x7A, 0];
        let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_wcspbrk_empty_set() {
        // An empty match set never matches.
        let ws1: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
        let ws2: [wchar_t; 1] = [0];
        let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_wcspbrk_first_char_matches() {
        let ws1: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let ws2: [wchar_t; 2] = [0x61, 0];
        let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
        assert_eq!(result.cast_const(), ws1.as_ptr());
    }

    #[test]
    fn test_wcspbrk_empty_haystack() {
        // An empty haystack never matches, regardless of the set.
        let ws1: [wchar_t; 1] = [0];
        let ws2: [wchar_t; 2] = [0x61, 0];
        let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
        assert!(result.is_null());
    }

    #[test]
    fn test_wcspbrk_duplicate_set() {
        // Duplicate characters in the set do not change the leftmost-match result.
        let ws1: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let ws2: [wchar_t; 4] = [0x7A, 0x7A, 0x62, 0];
        let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
        assert_eq!(result.cast_const(), unsafe { ws1.as_ptr().add(1) });
    }
}
