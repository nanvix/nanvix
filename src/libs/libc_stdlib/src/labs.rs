// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_long;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the absolute value of a long integer.
///
/// # Parameters
///
/// - `j`: Long integer value.
///
/// # Returns
///
/// The absolute value of `j`.
///
/// # Safety
///
/// This function is unsafe because it has C calling convention.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/labs.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn labs(j: c_long) -> c_long {
    if j < 0 {
        j.wrapping_neg()
    } else {
        j
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::labs;

    #[test]
    fn positive_value() {
        assert_eq!(unsafe { labs(42) }, 42);
    }

    #[test]
    fn negative_value() {
        assert_eq!(unsafe { labs(-42) }, 42);
    }

    #[test]
    fn zero_value() {
        assert_eq!(unsafe { labs(0) }, 0);
    }
}
