// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Multiplies a floating-point number by a power of two.
///
/// # Description
///
/// Computes `x * 2^exp`.
///
/// # Parameters
///
/// - `x`: Base value.
/// - `exp`: Exponent.
///
/// # Returns
///
/// The value `x * 2^exp`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ldexp(x: f64, exp: c_int) -> f64 {
    // Handle special cases.
    if x == 0.0 || x.is_nan() || x.to_bits() & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000 {
        return x;
    }

    let mut n: i64 = i64::from(exp);

    // Apply scaling in steps to handle large exponents.
    const SCALE_UP: f64 = f64::from_bits(0x7FE0_0000_0000_0000u64); // 2^1023
    const SCALE_DN: f64 = f64::from_bits(0x0010_0000_0000_0000u64); // 2^-1022

    let mut result: f64 = x;

    if n > 1023 {
        result *= SCALE_UP;
        n -= 1023;
        if n > 1023 {
            result *= SCALE_UP;
            n -= 1023;
            if n > 1023 {
                n = 1023;
            }
        }
    } else if n < -1022 {
        result *= SCALE_DN;
        n += 1022;
        if n < -1022 {
            result *= SCALE_DN;
            n += 1022;
            if n < -1022 {
                n = -1022;
            }
        }
    }

    let biased: u64 = (n + 1023) as u64;
    result * f64::from_bits(biased << 52)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert!((ldexp(1.0, 3) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_fraction() {
        assert!((ldexp(1.5, 2) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_exp() {
        assert!((ldexp(8.0, -3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(ldexp(0.0, 10), 0.0);
    }

    #[test]
    fn test_nan() {
        assert!(ldexp(f64::NAN, 5).is_nan());
    }
}
