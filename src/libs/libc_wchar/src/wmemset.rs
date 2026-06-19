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
/// Sets `n` wide characters in the array `s` to the value `c`.
///
/// # Parameters
///
/// - `s`: Pointer to the wide character array to fill.
/// - `c`: Wide character value to set.
/// - `n`: Number of wide characters to set.
///
/// # Return Value
///
/// Returns `s`.
///
/// # Safety
///
/// Behavior is undefined if `s` is null or if the buffer has fewer than `n` wide character
/// positions.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wmemset(s: *mut wchar_t, c: wchar_t, n: c_size_t) -> *mut wchar_t {
    debug_assert!(!s.is_null());

    let mut i: c_size_t = 0;
    while i < n {
        unsafe { *s.add(i as usize) = c };
        i += 1;
    }
    s
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wmemset_fill() {
        let mut buf: [wchar_t; 4] = [0; 4];
        let ret: *mut wchar_t = unsafe { wmemset(buf.as_mut_ptr(), 0x58, 4) };
        assert_eq!(ret, buf.as_mut_ptr());
        assert_eq!(buf, [0x58, 0x58, 0x58, 0x58]);
    }

    #[test]
    fn test_wmemset_zero_count() {
        let mut buf: [wchar_t; 3] = [-1; 3];
        unsafe { wmemset(buf.as_mut_ptr(), 0x41, 0) };
        assert_eq!(buf, [-1, -1, -1]);
    }
}
