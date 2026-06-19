// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `true` if the wide character `c` appears in the null-terminated delimiter set `delim`.
///
/// # Safety
///
/// `delim` must point to a valid, null-terminated wide string.
unsafe fn is_delim(c: wchar_t, delim: *const wchar_t) -> bool {
    let mut d: *const wchar_t = delim;
    while unsafe { *d } != 0 {
        if unsafe { *d } == c {
            return true;
        }
        d = unsafe { d.add(1) };
    }
    false
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Extracts the next token from the wide string `wcs`, delimited by any character in `delim`. The
/// parsing state is kept in `*ptr`, making this function reentrant.
///
/// # Safety
///
/// `delim` must point to a valid, null-terminated wide string, `ptr` must be a valid pointer, and
/// `wcs` (when non-null) must point to a valid, null-terminated wide string.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstok(
    wcs: *mut wchar_t,
    delim: *const wchar_t,
    ptr: *mut *mut wchar_t,
) -> *mut wchar_t {
    let mut s: *mut wchar_t = if wcs.is_null() { unsafe { *ptr } } else { wcs };
    if s.is_null() {
        return core::ptr::null_mut();
    }

    // Skip leading delimiters.
    while unsafe { *s } != 0 && unsafe { is_delim(*s, delim) } {
        s = unsafe { s.add(1) };
    }
    if unsafe { *s } == 0 {
        unsafe { *ptr = s };
        return core::ptr::null_mut();
    }

    let token: *mut wchar_t = s;

    // Find end of token.
    while unsafe { *s } != 0 && !unsafe { is_delim(*s, delim) } {
        s = unsafe { s.add(1) };
    }
    if unsafe { *s } != 0 {
        unsafe { *s = 0 };
        s = unsafe { s.add(1) };
    }
    unsafe { *ptr = s };
    token
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcstok_basic() {
        // "a,b,c" tokenized on ",".
        let mut buf: [wchar_t; 6] = [0x61, 0x2C, 0x62, 0x2C, 0x63, 0];
        let delim: [wchar_t; 2] = [0x2C, 0];
        let mut state: *mut wchar_t = core::ptr::null_mut();

        let t1: *mut wchar_t = unsafe { wcstok(buf.as_mut_ptr(), delim.as_ptr(), &mut state) };
        assert!(!t1.is_null());
        assert_eq!(unsafe { *t1 }, 0x61);

        let t2: *mut wchar_t = unsafe { wcstok(core::ptr::null_mut(), delim.as_ptr(), &mut state) };
        assert!(!t2.is_null());
        assert_eq!(unsafe { *t2 }, 0x62);

        let t3: *mut wchar_t = unsafe { wcstok(core::ptr::null_mut(), delim.as_ptr(), &mut state) };
        assert!(!t3.is_null());
        assert_eq!(unsafe { *t3 }, 0x63);

        let t4: *mut wchar_t = unsafe { wcstok(core::ptr::null_mut(), delim.as_ptr(), &mut state) };
        assert!(t4.is_null());
    }

    #[test]
    fn test_wcstok_leading_delimiters() {
        // Leading delimiters are skipped before the first token.
        let mut buf: [wchar_t; 5] = [0x2C, 0x2C, 0x61, 0x62, 0];
        let delim: [wchar_t; 2] = [0x2C, 0];
        let mut state: *mut wchar_t = core::ptr::null_mut();

        let t1: *mut wchar_t = unsafe { wcstok(buf.as_mut_ptr(), delim.as_ptr(), &mut state) };
        assert!(!t1.is_null());
        assert_eq!(unsafe { *t1 }, 0x61);
        assert_eq!(unsafe { *t1.add(1) }, 0x62);
    }

    #[test]
    fn test_wcstok_all_delimiters() {
        // A string made up solely of delimiters yields no tokens.
        let mut buf: [wchar_t; 3] = [0x2C, 0x2C, 0];
        let delim: [wchar_t; 2] = [0x2C, 0];
        let mut state: *mut wchar_t = core::ptr::null_mut();
        let t1: *mut wchar_t = unsafe { wcstok(buf.as_mut_ptr(), delim.as_ptr(), &mut state) };
        assert!(t1.is_null());
    }
}
