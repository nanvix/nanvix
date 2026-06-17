// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the floating-point remainder of `x / y` (single-precision).
///
/// # Parameters
///
/// - `x`: Dividend.
/// - `y`: Divisor.
///
/// # Returns
///
/// The floating-point remainder.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fmodf(x: f32, y: f32) -> f32 {
    if y == 0.0 || x.is_nan() || y.is_nan() {
        return f32::from_bits(0x7FC0_0000); // NaN
    }
    if x.to_bits() & 0x7F80_0000 == 0x7F80_0000 {
        return f32::from_bits(0x7FC0_0000); // NaN for inf
    }
    let n: f32 = crate::truncf::truncf(x / y);
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
        assert!((fmodf(5.3, 2.0) - 1.3).abs() < 1e-5);
    }

    #[test]
    fn test_negative() {
        assert!((fmodf(-5.3, 2.0) - (-1.3)).abs() < 1e-5);
    }

    #[test]
    fn test_zero_divisor() {
        assert!(fmodf(5.0, 0.0).is_nan());
    }
}
