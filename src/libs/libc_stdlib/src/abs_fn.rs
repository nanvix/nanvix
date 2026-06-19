// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the absolute value of an integer.
///
/// # Parameters
///
/// - `j`: Integer value.
///
/// # Returns
///
/// The absolute value of `j`. If the result cannot be represented (i.e., `j` equals `INT_MIN`),
/// the behavior is undefined.
///
/// # Safety
///
/// This function is unsafe because it has C calling convention.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/abs.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn abs(j: c_int) -> c_int {
    if j < 0 {
        j.wrapping_neg()
    } else {
        j
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::abs;

    #[test]
    fn positive_value() {
        assert_eq!(unsafe { abs(42) }, 42);
    }

    #[test]
    fn negative_value() {
        assert_eq!(unsafe { abs(-42) }, 42);
    }

    #[test]
    fn zero_value() {
        assert_eq!(unsafe { abs(0) }, 0);
    }
}
