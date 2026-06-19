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
/// Copies `n` wide characters from `src` to `dest`, handling overlapping memory regions correctly.
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
/// Behavior is undefined if `dest` or `src` is null, or if either buffer has fewer than `n` wide
/// character positions.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wmemmove(
    dest: *mut wchar_t,
    src: *const wchar_t,
    n: c_size_t,
) -> *mut wchar_t {
    debug_assert!(!dest.is_null());
    debug_assert!(!src.is_null());

    if (dest as *const wchar_t) <= src {
        // Copy forward.
        let mut i: c_size_t = 0;
        while i < n {
            unsafe { *dest.add(i as usize) = *src.add(i as usize) };
            i += 1;
        }
    } else {
        // Copy backward to handle overlap.
        let mut i: c_size_t = n;
        while i > 0 {
            i -= 1;
            unsafe { *dest.add(i as usize) = *src.add(i as usize) };
        }
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
    fn test_wmemmove_non_overlapping() {
        let src: [wchar_t; 3] = [0x41, 0x42, 0x43];
        let mut dest: [wchar_t; 3] = [0; 3];
        let ret: *mut wchar_t = unsafe { wmemmove(dest.as_mut_ptr(), src.as_ptr(), 3) };
        assert_eq!(ret, dest.as_mut_ptr());
        assert_eq!(dest, [0x41, 0x42, 0x43]);
    }

    #[test]
    fn test_wmemmove_overlap_forward() {
        // [1, 2, 3, 4, 5] — copy elements [0..3] to [1..4].
        let mut buf: [wchar_t; 5] = [1, 2, 3, 4, 5];
        unsafe {
            wmemmove(buf.as_mut_ptr().add(1), buf.as_ptr(), 3);
        }
        assert_eq!(buf, [1, 1, 2, 3, 5]);
    }

    #[test]
    fn test_wmemmove_overlap_backward() {
        // [1, 2, 3, 4, 5] — copy elements [1..4] to [0..3].
        let mut buf: [wchar_t; 5] = [1, 2, 3, 4, 5];
        unsafe {
            wmemmove(buf.as_mut_ptr(), buf.as_ptr().add(1), 3);
        }
        assert_eq!(buf, [2, 3, 4, 4, 5]);
    }

    #[test]
    fn test_wmemmove_zero_count() {
        let src: [wchar_t; 2] = [0x41, 0x42];
        let mut dest: [wchar_t; 2] = [-1; 2];
        unsafe { wmemmove(dest.as_mut_ptr(), src.as_ptr(), 0) };
        assert_eq!(dest, [-1, -1]);
    }
}
