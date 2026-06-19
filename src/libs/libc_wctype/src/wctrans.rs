// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    cstring::c_str_eq,
    wctrans_t::wctrans_t,
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Constants
//==================================================================================================

/// Descriptor for the `tolower` wide-character mapping.
pub const WCTRANS_TOLOWER: wctrans_t = 1;

/// Descriptor for the `toupper` wide-character mapping.
pub const WCTRANS_TOUPPER: wctrans_t = 2;

//==================================================================================================
// Standalone Functions
//==================================================================================================

unsafe fn mapping_from_name(charclass: *const c_char) -> wctrans_t {
    if charclass.is_null() {
        return 0;
    }

    if unsafe { c_str_eq(charclass, b"tolower") } {
        WCTRANS_TOLOWER
    } else if unsafe { c_str_eq(charclass, b"toupper") } {
        WCTRANS_TOUPPER
    } else {
        0
    }
}

///
/// # Description
///
/// Defines a wide-character mapping descriptor.
///
/// # Parameters
///
/// - `charclass`: Pointer to a null-terminated mapping name.
///
/// # Return Value
///
/// A non-zero mapping descriptor for valid names, or zero otherwise.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `charclass`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wctrans(charclass: *const c_char) -> wctrans_t {
    unsafe { mapping_from_name(charclass) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        wctrans,
        WCTRANS_TOLOWER,
        WCTRANS_TOUPPER,
    };

    #[test]
    fn test_wctrans_valid_mappings() {
        let tolower = b"tolower\0";
        let toupper = b"toupper\0";
        assert_eq!(unsafe { wctrans(tolower.as_ptr().cast()) }, WCTRANS_TOLOWER);
        assert_eq!(unsafe { wctrans(toupper.as_ptr().cast()) }, WCTRANS_TOUPPER);
    }

    #[test]
    fn test_wctrans_invalid_mapping() {
        let invalid = b"titlecase\0";
        assert_eq!(unsafe { wctrans(invalid.as_ptr().cast()) }, 0);
    }

    #[test]
    fn test_wctrans_null_mapping() {
        assert_eq!(unsafe { wctrans(::core::ptr::null()) }, 0);
    }
}
