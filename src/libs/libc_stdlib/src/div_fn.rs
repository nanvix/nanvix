// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Structures
//==================================================================================================

/// Result type for the `div()` function containing quotient and remainder.
#[repr(C)]
pub struct div_t {
    pub quot: c_int,
    pub rem: c_int,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the quotient and remainder of an integer division.
///
/// # Parameters
///
/// - `numer`: Numerator.
/// - `denom`: Denominator.
///
/// # Returns
///
/// A `div_t` structure containing the quotient and remainder.
///
/// # Safety
///
/// This function is unsafe because it has C calling convention. The caller must ensure that
/// `denom` is not zero.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/div.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn div(numer: c_int, denom: c_int) -> div_t {
    div_t {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::div;

    #[test]
    fn positive_division() {
        let result = unsafe { div(10, 3) };
        assert_eq!(result.quot, 3);
        assert_eq!(result.rem, 1);
    }

    #[test]
    fn negative_dividend() {
        let result = unsafe { div(-10, 3) };
        assert_eq!(result.quot, -3);
        assert_eq!(result.rem, -1);
    }
}
