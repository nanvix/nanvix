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
/// Copies at most `n` wide characters from `src` to `dest`. If `src` is shorter than `n`
/// characters, the remainder of `dest` is padded with null wide characters.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination wide string buffer.
/// - `src`: Pointer to the source null-terminated wide string.
/// - `n`: Maximum number of wide characters to copy.
///
/// # Return Value
///
/// Returns `dest`.
///
/// # Safety
///
/// Behavior is undefined if `dest` or `src` is null, or if the destination buffer has fewer than
/// `n` wide character positions.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsncpy(
    dest: *mut wchar_t,
    src: *const wchar_t,
    n: c_size_t,
) -> *mut wchar_t {
    debug_assert!(!dest.is_null());
    debug_assert!(!src.is_null());

    let mut i: c_size_t = 0;
    // Copy characters from src.
    while i < n {
        let c: wchar_t = unsafe { *src.add(i as usize) };
        unsafe { *dest.add(i as usize) = c };
        if c == 0 {
            i += 1;
            break;
        }
        i += 1;
    }
    // Pad remainder with null wide characters.
    while i < n {
        unsafe { *dest.add(i as usize) = 0 };
        i += 1;
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
    fn test_wcsncpy_exact() {
        // Source fits exactly in n (including null).
        let src: [wchar_t; 3] = [0x41, 0x42, 0];
        let mut dest: [wchar_t; 3] = [-1; 3];
        unsafe { wcsncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
        assert_eq!(dest, [0x41, 0x42, 0]);
    }

    #[test]
    fn test_wcsncpy_shorter_src() {
        // Source is shorter than n: remainder should be zero-padded.
        let src: [wchar_t; 2] = [0x41, 0];
        let mut dest: [wchar_t; 5] = [-1; 5];
        unsafe { wcsncpy(dest.as_mut_ptr(), src.as_ptr(), 5) };
        assert_eq!(dest, [0x41, 0, 0, 0, 0]);
    }

    #[test]
    fn test_wcsncpy_longer_src() {
        // Source is longer than n: result is not null-terminated.
        let src: [wchar_t; 6] = [0x41, 0x42, 0x43, 0x44, 0x45, 0];
        let mut dest: [wchar_t; 3] = [-1; 3];
        unsafe { wcsncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
        assert_eq!(dest, [0x41, 0x42, 0x43]);
    }
}
