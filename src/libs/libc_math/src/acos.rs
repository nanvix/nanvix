// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const PI: f64 = core::f64::consts::PI;
const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the arccosine of `x`.
///
/// # Parameters
///
/// - `x`: Input value in `[-1, 1]`.
///
/// # Returns
///
/// The arccosine of `x` in radians in `[0, PI]`. NaN if `|x| > 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn acos(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let abs_x: f64 = f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF);
    if abs_x > 1.0 {
        return f64::from_bits(0x7FF8_0000_0000_0000);
    }
    if x == 1.0 {
        return 0.0;
    }
    if x == -1.0 {
        return PI;
    }
    FRAC_PI_2 - crate::asin::asin(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((acos(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert!((acos(0.0) - FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn test_negative_one() {
        assert!((acos(-1.0) - PI).abs() < 1e-10);
    }

    #[test]
    fn test_half() {
        let pi_over_3: f64 = PI / 3.0;
        assert!((acos(0.5) - pi_over_3).abs() < 1e-10);
    }

    #[test]
    fn test_out_of_range() {
        assert!(acos(1.5).is_nan());
    }
}
