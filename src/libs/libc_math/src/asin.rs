// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the arcsine of `x`.
///
/// # Description
///
/// Returns the angle in radians whose sine is `x`. Uses identity
/// `asin(x) = atan(x / sqrt(1 - x^2))` for `|x| <= 0.5` and
/// `asin(x) = PI/2 - 2*asin(sqrt((1-|x|)/2))` for `|x| > 0.5`.
///
/// # Parameters
///
/// - `x`: Input value in `[-1, 1]`.
///
/// # Returns
///
/// The arcsine of `x` in radians in `[-PI/2, PI/2]`. NaN if `|x| > 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn asin(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let abs_x: f64 = f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF);
    if abs_x > 1.0 {
        return f64::from_bits(0x7FF8_0000_0000_0000); // NaN
    }
    if abs_x == 1.0 {
        return if x > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
    }

    if abs_x <= 0.5 {
        let denom: f64 = crate::sqrt::sqrt(1.0 - x * x);
        if denom == 0.0 {
            return if x > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
        }
        crate::atan::atan(x / denom)
    } else {
        // For |x| > 0.5, use identity for better accuracy.
        let half: f64 = (1.0 - abs_x) * 0.5;
        let s: f64 = crate::sqrt::sqrt(half);
        let result: f64 = FRAC_PI_2 - 2.0 * crate::atan::atan(s / crate::sqrt::sqrt(1.0 - half));
        if x < 0.0 {
            -result
        } else {
            result
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((asin(0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_half() {
        let pi_over_6: f64 = 3.141_592_653_589_793 / 6.0;
        assert!((asin(0.5) - pi_over_6).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((asin(1.0) - FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn test_negative_one() {
        assert!((asin(-1.0) - (-FRAC_PI_2)).abs() < 1e-10);
    }

    #[test]
    fn test_out_of_range() {
        assert!(asin(1.5).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(asin(f64::NAN).is_nan());
    }
}
