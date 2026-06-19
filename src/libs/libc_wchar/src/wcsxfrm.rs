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
mod test {
    use super::*;

    #[test]
    fn test_wcsxfrm_copies_identity_transform() {
        let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let mut dest: [wchar_t; 4] = [-1; 4];
        let len: c_size_t = unsafe {
            wcsxfrm(
                dest.as_mut_ptr(),
                src.as_ptr(),
                c_size_t::try_from(dest.len()).expect("destination length should fit in c_size_t"),
            )
        };
        assert_eq!(len, 3);
        assert_eq!(dest, src);
    }

    #[test]
    fn test_wcsxfrm_zero_size_returns_required_length() {
        let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let len: c_size_t = unsafe { wcsxfrm(core::ptr::null_mut(), src.as_ptr(), 0) };
        assert_eq!(len, 3);
    }

    #[test]
    fn test_wcsxfrm_truncates_but_terminates_when_space_exists() {
        let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let mut dest: [wchar_t; 2] = [-1; 2];
        let len: c_size_t = unsafe {
            wcsxfrm(
                dest.as_mut_ptr(),
                src.as_ptr(),
                c_size_t::try_from(dest.len()).expect("destination length should fit in c_size_t"),
            )
        };
        assert_eq!(len, 3);
        assert_eq!(dest, [0x61, 0]);
    }
}
