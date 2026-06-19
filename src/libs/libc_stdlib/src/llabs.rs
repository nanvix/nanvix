// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_longlong;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the absolute value of a long long integer.
///
/// # Parameters
///
/// - `j`: Long long integer value.
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
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/llabs.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn llabs(j: c_longlong) -> c_longlong {
    if j < 0 {
        j.wrapping_neg()
    } else {
        j
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::llabs;

    #[test]
    fn positive_value() {
        assert_eq!(unsafe { llabs(42) }, 42);
    }

    #[test]
    fn negative_value() {
        assert_eq!(unsafe { llabs(-42) }, 42);
    }

    #[test]
    fn large_value() {
        assert_eq!(unsafe { llabs(-1_000_000_000_000) }, 1_000_000_000_000);
    }
}
