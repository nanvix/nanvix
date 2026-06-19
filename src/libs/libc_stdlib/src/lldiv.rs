// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_longlong;

//==================================================================================================
// Structures
//==================================================================================================

/// Result type for the `lldiv()` function containing quotient and remainder.
#[repr(C)]
pub struct lldiv_t {
    pub quot: c_longlong,
    pub rem: c_longlong,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the quotient and remainder of a long long integer division.
///
/// # Parameters
///
/// - `numer`: Numerator.
/// - `denom`: Denominator.
///
/// # Returns
///
/// An `lldiv_t` structure containing the quotient and remainder.
///
/// # Safety
///
/// This function is unsafe because it has C calling convention. The caller must ensure that
/// `denom` is not zero.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/lldiv.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn lldiv(numer: c_longlong, denom: c_longlong) -> lldiv_t {
    lldiv_t {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::lldiv;

    #[test]
    fn positive_division() {
        let result = unsafe { lldiv(100, 7) };
        assert_eq!(result.quot, 14);
        assert_eq!(result.rem, 2);
    }

    #[test]
    fn large_values() {
        let result = unsafe { lldiv(1_000_000_000_000, 7) };
        assert_eq!(result.quot, 142_857_142_857);
        assert_eq!(result.rem, 1);
    }
}
