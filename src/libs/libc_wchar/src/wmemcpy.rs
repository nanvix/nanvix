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
mod test;
