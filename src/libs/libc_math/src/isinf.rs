// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Tests whether a single-precision value is infinite.
///
/// # Parameters
///
/// - `x`: Value to test.
///
/// # Returns
///
/// Non-zero if `x` is infinite, zero otherwise.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __isinff(x: f32) -> c_int {
    let bits: u32 = x.to_bits();
    if bits & 0x7FFF_FFFF == 0x7F80_0000 {
        1
    } else {
        0
    }
}

/// Tests whether a double-precision value is infinite.
///
/// # Parameters
///
/// - `x`: Value to test.
///
/// # Returns
///
/// Non-zero if `x` is infinite, zero otherwise.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __isinfd(x: f64) -> c_int {
    let bits: u64 = x.to_bits();
    if bits & 0x7FFF_FFFF_FFFF_FFFF == 0x7FF0_0000_0000_0000 {
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
    fn test_isinff() {
        assert_eq!(__isinff(f32::INFINITY), 1);
        assert_eq!(__isinff(f32::NEG_INFINITY), 1);
        assert_eq!(__isinff(1.0), 0);
        assert_eq!(__isinff(f32::NAN), 0);
    }

    #[test]
    fn test_isinfd() {
        assert_eq!(__isinfd(f64::INFINITY), 1);
        assert_eq!(__isinfd(f64::NEG_INFINITY), 1);
        assert_eq!(__isinfd(1.0), 0);
        assert_eq!(__isinfd(f64::NAN), 0);
    }
}
