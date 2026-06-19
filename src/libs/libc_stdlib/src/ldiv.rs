// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_long;

//==================================================================================================
// Structures
//==================================================================================================

/// Result type for the `ldiv()` function containing quotient and remainder.
#[repr(C)]
pub struct ldiv_t {
    pub quot: c_long,
    pub rem: c_long,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the quotient and remainder of a long integer division.
///
/// # Parameters
///
/// - `numer`: Numerator.
/// - `denom`: Denominator.
///
/// # Returns
///
/// An `ldiv_t` structure containing the quotient and remainder.
///
/// # Safety
///
/// This function is unsafe because it has C calling convention. The caller must ensure that
/// `denom` is not zero.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/ldiv.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ldiv(numer: c_long, denom: c_long) -> ldiv_t {
    ldiv_t {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::ldiv;

    #[test]
    fn positive_division() {
        let result = unsafe { ldiv(17, 5) };
        assert_eq!(result.quot, 3);
        assert_eq!(result.rem, 2);
    }

    #[test]
    fn negative_dividend() {
        let result = unsafe { ldiv(-17, 5) };
        assert_eq!(result.quot, -3);
        assert_eq!(result.rem, -2);
    }
}
