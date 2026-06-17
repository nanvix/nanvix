// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;
const FRAC_PI_4: f64 = core::f64::consts::FRAC_PI_4;

//==================================================================================================
// Internal Helpers
//==================================================================================================

/// Polynomial approximation of atan(x) for |x| <= 1.
fn atan_poly(x: f64) -> f64 {
    let x2: f64 = x * x;
    // Polynomial: x - x^3/3 + x^5/5 - x^7/7 + x^9/9 - x^11/11 + x^13/13
    x * (1.0
        + x2 * (-1.0 / 3.0
            + x2 * (1.0 / 5.0
                + x2 * (-1.0 / 7.0
                    + x2 * (1.0 / 9.0
                        + x2 * (-1.0 / 11.0
                            + x2 * (1.0 / 13.0 + x2 * (-1.0 / 15.0 + x2 * (1.0 / 17.0)))))))))
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the arctangent of `x`.
///
/// # Description
///
/// Uses argument reduction and polynomial approximation.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The arctangent of `x` in radians, in the range `[-PI/2, PI/2]`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn atan(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.to_bits() == 0x7FF0_0000_0000_0000 {
        return FRAC_PI_2;
    }
    if x.to_bits() == 0xFFF0_0000_0000_0000 {
        return -FRAC_PI_2;
    }

    let abs_x: f64 = f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF);
    let sign: f64 = if x < 0.0 { -1.0 } else { 1.0 };

    if abs_x > 1.0 {
        // Use identity: atan(x) = PI/2 - atan(1/x) for |x| > 1.
        sign * (FRAC_PI_2 - atan_poly(1.0 / abs_x))
    } else if abs_x > 0.414_213_562_373_095 {
        // Use identity: atan(x) = PI/4 + atan((x-1)/(x+1)) for x near 1.
        let t: f64 = (abs_x - 1.0) / (abs_x + 1.0);
        sign * (FRAC_PI_4 + atan_poly(t))
    } else {
        atan_poly(x)
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
        assert!((atan(0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((atan(1.0) - FRAC_PI_4).abs() < 1e-10);
    }

    #[test]
    fn test_negative_one() {
        assert!((atan(-1.0) - (-FRAC_PI_4)).abs() < 1e-10);
    }

    #[test]
    fn test_large() {
        assert!((atan(1000.0) - FRAC_PI_2).abs() < 1e-3);
    }

    #[test]
    fn test_infinity() {
        assert!((atan(f64::INFINITY) - FRAC_PI_2).abs() < 1e-10);
        assert!((atan(f64::NEG_INFINITY) - (-FRAC_PI_2)).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(atan(f64::NAN).is_nan());
    }
}
