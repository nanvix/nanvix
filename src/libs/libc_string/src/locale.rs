// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    strcoll::strcoll,
    strxfrm::strxfrm,
};
use ::sysapi::{
    ffi::{
        c_char,
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
/// Compares two strings using the collating sequence of the C/POSIX locale.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated string.
/// - `s2`: Pointer to the second null-terminated string.
/// - `locale`: Locale to use (ignored; only the C/POSIX locale is supported).
///
/// # Returns
///
/// A negative, zero, or positive value if `s1` orders before, equal to, or after `s2`.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `s1` and `s2`, which must point
/// to valid null-terminated strings.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcoll_l(
    s1: *const c_char,
    s2: *const c_char,
    _locale: *mut c_void,
) -> c_int {
    unsafe { strcoll(s1, s2) }
}

/// # Description
///
/// Transforms a string so that the result of `strcmp()` on two transformed strings matches the
/// result of `strcoll()` on the originals. In the C/POSIX locale the transformation is the identity
/// copy.
///
/// # Parameters
///
/// - `dest`: Destination buffer for the transformed string.
/// - `src`: Pointer to the null-terminated source string.
/// - `n`: Maximum number of bytes to write to `dest`, including the null terminator.
/// - `locale`: Locale to use (ignored; only the C/POSIX locale is supported).
///
/// # Returns
///
/// The length of the transformed string (excluding the null terminator).
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `dest` and `src`, which must be
/// valid for the requested operation.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strxfrm_l(
    dest: *mut c_char,
    src: *const c_char,
    n: c_size_t,
    _locale: *mut c_void,
) -> c_size_t {
    unsafe { strxfrm(dest, src, n) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;
    use ::std::vec::Vec;

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0);
        v
    }

    #[test]
    fn test_strcoll_l_orders_like_strcmp() {
        let locale: *mut c_void = ::core::ptr::null_mut();
        let a: Vec<c_char> = make_c_string(b"abc");
        let b: Vec<c_char> = make_c_string(b"abd");

        assert!(unsafe { strcoll_l(a.as_ptr(), b.as_ptr(), locale) } < 0);
        assert!(unsafe { strcoll_l(b.as_ptr(), a.as_ptr(), locale) } > 0);
        assert_eq!(unsafe { strcoll_l(a.as_ptr(), a.as_ptr(), locale) }, 0);
    }

    #[test]
    fn test_strxfrm_l_is_identity_in_c_locale() {
        let locale: *mut c_void = ::core::ptr::null_mut();
        let src: Vec<c_char> = make_c_string(b"abc");
        let mut dest: [c_char; 8] = [0; 8];
        let capacity: c_size_t = c_size_t::try_from(dest.len()).expect("capacity fits");

        let len: c_size_t = unsafe { strxfrm_l(dest.as_mut_ptr(), src.as_ptr(), capacity, locale) };
        assert_eq!(len, 3);
        assert_eq!(&dest[..4], &src[..4]);
    }
}
