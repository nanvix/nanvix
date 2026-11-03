// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    wchar_t::wchar_t,
    wcslen::wcslen,
};
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Transforms a wide-character string for locale-aware collation.
///
/// Nanvix currently supports the C/POSIX locale, so the transformed string is identical to `src`.
/// The return value is the full transformed length, excluding the null terminator.
///
/// # Safety
///
/// `src` must point to a valid null-terminated wide-character string. If `n` is greater than zero,
/// `dest` must point to an array of at least `n` wide characters.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsxfrm(dest: *mut wchar_t, src: *const wchar_t, n: c_size_t) -> c_size_t {
    let len: c_size_t = unsafe { wcslen(src) };

    if n == 0 {
        return len;
    }

    let mut i: c_size_t = 0;
    while i + 1 < n && i < len {
        unsafe { *dest.add(i as usize) = *src.add(i as usize) };
        i += 1;
    }
    unsafe { *dest.add(i as usize) = 0 };
    len
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
