// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Tests whether the sign bit of a single-precision value is set.
///
/// # Parameters
///
/// - `x`: Value to test.
///
/// # Returns
///
/// Non-zero if the sign bit is set, zero otherwise.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __signbitf(x: f32) -> c_int {
    if x.to_bits() & 0x8000_0000 != 0 {
        1
    } else {
        0
    }
}

/// Tests whether the sign bit of a double-precision value is set.
///
/// # Parameters
///
/// - `x`: Value to test.
///
/// # Returns
///
/// Non-zero if the sign bit is set, zero otherwise.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __signbitd(x: f64) -> c_int {
    if x.to_bits() & 0x8000_0000_0000_0000 != 0 {
        1
    } else {
        0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_signbitf() {
        assert_eq!(__signbitf(1.0), 0);
        assert_eq!(__signbitf(-1.0), 1);
        assert_eq!(__signbitf(-0.0), 1);
        assert_eq!(__signbitf(0.0), 0);
    }

    #[test]
    fn test_signbitd() {
        assert_eq!(__signbitd(1.0), 0);
        assert_eq!(__signbitd(-1.0), 1);
        assert_eq!(__signbitd(-0.0), 1);
        assert_eq!(__signbitd(0.0), 0);
    }
}
