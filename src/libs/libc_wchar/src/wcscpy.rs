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
/// Copies a wide string from `src` to `dest`, including the null terminator.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination wide string buffer.
/// - `src`: Pointer to the source null-terminated wide string.
///
/// # Return Value
///
/// Returns `dest`.
///
/// # Safety
///
/// Behavior is undefined if `dest` or `src` is null, or if the destination buffer is not large
/// enough to hold the source string including its null terminator.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcscpy(dest: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    debug_assert!(!dest.is_null());
    debug_assert!(!src.is_null());

    let mut i: c_size_t = 0;
    loop {
        let c: wchar_t = unsafe { *src.add(i as usize) };
        unsafe { *dest.add(i as usize) = c };
        if c == 0 {
            break;
        }
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
    fn test_wcscpy_empty() {
        let src: [wchar_t; 1] = [0];
        let mut dest: [wchar_t; 1] = [-1];
        unsafe { wcscpy(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dest[0], 0);
    }

    #[test]
    fn test_wcscpy_hello() {
        let src: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
        let mut dest: [wchar_t; 6] = [0; 6];
        let ret: *mut wchar_t = unsafe { wcscpy(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(ret, dest.as_mut_ptr());
        assert_eq!(dest, src);
    }
}
