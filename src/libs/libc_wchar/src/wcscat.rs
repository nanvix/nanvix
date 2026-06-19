// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Appends the wide string `src` to the end of the wide string `dest`, including the null
/// terminator.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination null-terminated wide string.
/// - `src`: Pointer to the source null-terminated wide string.
///
/// # Return Value
///
/// Returns `dest`.
///
/// # Safety
///
/// Behavior is undefined if `dest` or `src` is null, or if the destination buffer is not large
/// enough to hold the concatenated result.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcscat(dest: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    debug_assert!(!dest.is_null());
    debug_assert!(!src.is_null());

    // Find the end of dest.
    let mut d: c_size_t = 0;
    while unsafe { *dest.add(d as usize) } != 0 {
        d += 1;
    }

    // Copy src to the end of dest.
    let mut s: c_size_t = 0;
    loop {
        let c: wchar_t = unsafe { *src.add(s as usize) };
        unsafe { *dest.add(d as usize) = c };
        if c == 0 {
            break;
        }
        d += 1;
        s += 1;
    }
    dest
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcscat_basic() {
        // "AB" + "CD" = "ABCD"
        let mut dest: [wchar_t; 8] = [0x41, 0x42, 0, 0, 0, 0, 0, 0];
        let src: [wchar_t; 3] = [0x43, 0x44, 0];
        let ret: *mut wchar_t = unsafe { wcscat(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(ret, dest.as_mut_ptr());
        assert_eq!(dest[0], 0x41);
        assert_eq!(dest[1], 0x42);
        assert_eq!(dest[2], 0x43);
        assert_eq!(dest[3], 0x44);
        assert_eq!(dest[4], 0);
    }

    #[test]
    fn test_wcscat_empty_src() {
        let mut dest: [wchar_t; 4] = [0x41, 0, 0, 0];
        let src: [wchar_t; 1] = [0];
        unsafe { wcscat(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dest[0], 0x41);
        assert_eq!(dest[1], 0);
    }

    #[test]
    fn test_wcscat_empty_dest() {
        let mut dest: [wchar_t; 4] = [0; 4];
        let src: [wchar_t; 3] = [0x41, 0x42, 0];
        unsafe { wcscat(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dest[0], 0x41);
        assert_eq!(dest[1], 0x42);
        assert_eq!(dest[2], 0);
    }
}
