// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the floating-point remainder of `x / y`.
///
/// # Description
///
/// The result has the same sign as `x` and magnitude less than that of `y`.
///
/// # Parameters
///
/// - `x`: Dividend.
/// - `y`: Divisor.
///
/// # Returns
///
/// The floating-point remainder `x - n*y` where `n = trunc(x/y)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
    if y == 0.0 || x.is_nan() || y.is_nan() {
        return f64::from_bits(0x7FF8_0000_0000_0000); // NaN
    }
    if x.to_bits() & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000 {
        return f64::from_bits(0x7FF8_0000_0000_0000); // NaN for inf
    }
    let n: f64 = crate::trunc::trunc(x / y);
    x - n * y
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert!((fmod(5.3, 2.0) - 1.3).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((fmod(-5.3, 2.0) - (-1.3)).abs() < 1e-10);
    }

    #[test]
    fn test_exact() {
        assert!((fmod(6.0, 3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_zero_divisor() {
        assert!(fmod(5.0, 0.0).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(fmod(f64::NAN, 2.0).is_nan());
    }
}
