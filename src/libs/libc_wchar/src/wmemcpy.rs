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
/// Copies `n` wide characters from `src` to `dest`. The memory regions must not overlap.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination wide character array.
/// - `src`: Pointer to the source wide character array.
/// - `n`: Number of wide characters to copy.
///
/// # Return Value
///
/// Returns `dest`.
///
/// # Safety
///
/// Behavior is undefined if `dest` or `src` is null, if the memory regions overlap, or if either
/// buffer has fewer than `n` wide character positions.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wmemcpy(
    dest: *mut wchar_t,
    src: *const wchar_t,
    n: c_size_t,
) -> *mut wchar_t {
    debug_assert!(!dest.is_null());
    debug_assert!(!src.is_null());

    let mut i: c_size_t = 0;
    while i < n {
        unsafe { *dest.add(i as usize) = *src.add(i as usize) };
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
    fn test_wmemcpy_basic() {
        let src: [wchar_t; 3] = [0x41, 0x42, 0x43];
        let mut dest: [wchar_t; 3] = [0; 3];
        let ret: *mut wchar_t = unsafe { wmemcpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
        assert_eq!(ret, dest.as_mut_ptr());
        assert_eq!(dest, [0x41, 0x42, 0x43]);
    }

    #[test]
    fn test_wmemcpy_zero() {
        let src: [wchar_t; 3] = [0x41, 0x42, 0x43];
        let mut dest: [wchar_t; 3] = [-1; 3];
        unsafe { wmemcpy(dest.as_mut_ptr(), src.as_ptr(), 0) };
        assert_eq!(dest, [-1, -1, -1]);
    }
}
