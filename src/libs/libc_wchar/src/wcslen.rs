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
/// Computes the length of a wide string.
///
/// # Parameters
///
/// - `s`: Pointer to a null-terminated wide string.
///
/// # Return Value
///
/// The number of wide characters preceding the null terminator.
///
/// # Safety
///
/// Behavior is undefined if `s` is null or does not point to a valid null-terminated wide string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcslen(s: *const wchar_t) -> c_size_t {
    debug_assert!(!s.is_null());

    let mut i: c_size_t = 0;
    while unsafe { *s.add(i as usize) } != 0 {
        i += 1;
    }
    i
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcslen_empty() {
        let s: [wchar_t; 1] = [0];
        assert_eq!(unsafe { wcslen(s.as_ptr()) }, 0);
    }

    #[test]
    fn test_wcslen_hello() {
        // "hello" as wide characters
        let s: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
        assert_eq!(unsafe { wcslen(s.as_ptr()) }, 5);
    }

    #[test]
    fn test_wcslen_single() {
        let s: [wchar_t; 2] = [0x41, 0];
        assert_eq!(unsafe { wcslen(s.as_ptr()) }, 1);
    }
}
