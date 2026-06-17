// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// IEEE 754 classification: Not a Number.
const FP_NAN: c_int = 0;
/// IEEE 754 classification: Infinity.
const FP_INFINITE: c_int = 1;
/// IEEE 754 classification: Zero.
const FP_ZERO: c_int = 2;
/// IEEE 754 classification: Subnormal.
const FP_SUBNORMAL: c_int = 3;
/// IEEE 754 classification: Normal.
const FP_NORMAL: c_int = 4;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Classifies a single-precision floating-point value.
///
/// # Parameters
///
/// - `x`: Value to classify.
///
/// # Returns
///
/// `FP_NAN` (0), `FP_INFINITE` (1), `FP_ZERO` (2), `FP_SUBNORMAL` (3), or `FP_NORMAL` (4).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __fpclassifyf(x: f32) -> c_int {
    let bits: u32 = x.to_bits();
    let exp: u32 = bits & 0x7F80_0000;
    let mantissa: u32 = bits & 0x007F_FFFF;

    if exp == 0x7F80_0000 {
        if mantissa != 0 {
            FP_NAN
        } else {
            FP_INFINITE
        }
    } else if exp == 0 {
        if mantissa == 0 {
            FP_ZERO
        } else {
            FP_SUBNORMAL
        }
    } else {
        FP_NORMAL
    }
}

/// Classifies a double-precision floating-point value.
///
/// # Parameters
///
/// - `x`: Value to classify.
///
/// # Returns
///
/// `FP_NAN` (0), `FP_INFINITE` (1), `FP_ZERO` (2), `FP_SUBNORMAL` (3), or `FP_NORMAL` (4).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __fpclassifyd(x: f64) -> c_int {
    let bits: u64 = x.to_bits();
    let exp: u64 = bits & 0x7FF0_0000_0000_0000;
    let mantissa: u64 = bits & 0x000F_FFFF_FFFF_FFFF;

    if exp == 0x7FF0_0000_0000_0000 {
        if mantissa != 0 {
            FP_NAN
        } else {
            FP_INFINITE
        }
    } else if exp == 0 {
        if mantissa == 0 {
            FP_ZERO
        } else {
            FP_SUBNORMAL
        }
    } else {
        FP_NORMAL
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_fpclassifyf() {
        assert_eq!(__fpclassifyf(1.0), FP_NORMAL);
        assert_eq!(__fpclassifyf(0.0), FP_ZERO);
        assert_eq!(__fpclassifyf(f32::INFINITY), FP_INFINITE);
        assert_eq!(__fpclassifyf(f32::NAN), FP_NAN);
        assert_eq!(__fpclassifyf(1.0e-40), FP_SUBNORMAL);
    }

    #[test]
    fn test_fpclassifyd() {
        assert_eq!(__fpclassifyd(1.0), FP_NORMAL);
        assert_eq!(__fpclassifyd(0.0), FP_ZERO);
        assert_eq!(__fpclassifyd(f64::INFINITY), FP_INFINITE);
        assert_eq!(__fpclassifyd(f64::NAN), FP_NAN);
        assert_eq!(__fpclassifyd(5.0e-324), FP_SUBNORMAL);
    }
}
