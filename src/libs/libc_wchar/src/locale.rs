// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    wchar_t::wchar_t,
    wcscoll::wcscoll,
    wcsxfrm::wcsxfrm,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Nanvix supports only the C/POSIX locale, so each `*_l` function ignores its `locale_t` argument
// and delegates to its non-`_l` counterpart.

/// # Description
///
/// Compares two wide-character strings using the collating sequence of the C/POSIX locale.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated wide-character string.
/// - `s2`: Pointer to the second null-terminated wide-character string.
/// - `locale`: Locale to use (ignored; only the C/POSIX locale is supported).
///
/// # Returns
///
/// A negative, zero, or positive value if `s1` orders before, equal to, or after `s2`.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `s1` and `s2`, which must point
/// to valid null-terminated wide-character strings.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcscoll_l(
    s1: *const wchar_t,
    s2: *const wchar_t,
    _locale: *mut c_void,
) -> c_int {
    unsafe { wcscoll(s1, s2) }
}

/// # Description
///
/// Transforms a wide-character string so that the result of `wcscmp()` on two transformed strings
/// matches the result of `wcscoll()` on the originals. In the C/POSIX locale the transformation is
/// the identity copy.
///
/// # Parameters
///
/// - `dest`: Destination buffer for the transformed wide-character string.
/// - `src`: Pointer to the null-terminated source wide-character string.
/// - `n`: Maximum number of wide characters to write to `dest`, including the null terminator.
/// - `locale`: Locale to use (ignored; only the C/POSIX locale is supported).
///
/// # Returns
///
/// The length of the transformed wide-character string (excluding the null terminator).
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `dest` and `src`, which must be
/// valid for the requested operation.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsxfrm_l(
    dest: *mut wchar_t,
    src: *const wchar_t,
    n: c_size_t,
    _locale: *mut c_void,
) -> c_size_t {
    unsafe { wcsxfrm(dest, src, n) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcscoll_l_orders_like_wcscmp() {
        let locale: *mut c_void = ::core::ptr::null_mut();
        let a: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let b: [wchar_t; 4] = [0x61, 0x62, 0x64, 0];

        assert!(unsafe { wcscoll_l(a.as_ptr(), b.as_ptr(), locale) } < 0);
        assert!(unsafe { wcscoll_l(b.as_ptr(), a.as_ptr(), locale) } > 0);
        assert_eq!(unsafe { wcscoll_l(a.as_ptr(), a.as_ptr(), locale) }, 0);
    }

    #[test]
    fn test_wcsxfrm_l_is_identity_in_c_locale() {
        let locale: *mut c_void = ::core::ptr::null_mut();
        let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let mut dest: [wchar_t; 8] = [-1; 8];
        let capacity: c_size_t =
            c_size_t::try_from(dest.len()).expect("destination length should fit in c_size_t");

        let len: c_size_t = unsafe { wcsxfrm_l(dest.as_mut_ptr(), src.as_ptr(), capacity, locale) };
        assert_eq!(len, 3);
        assert_eq!(&dest[..4], &src[..4]);
    }
}
