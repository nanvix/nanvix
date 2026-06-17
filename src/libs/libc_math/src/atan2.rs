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

/// Computes the four-quadrant arctangent of `y / x`.
///
/// # Description
///
/// Returns the angle in radians between the positive x-axis and the point `(x, y)`.
///
/// # Parameters
///
/// - `y`: Y-coordinate.
/// - `x`: X-coordinate.
///
/// # Returns
///
/// The angle in radians in the range `[-PI, PI]`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn atan2(y: f64, x: f64) -> f64 {
    // Handle NaN.
    if x.is_nan() || y.is_nan() {
        return f64::from_bits(0x7FF8_0000_0000_0000);
    }

    // Handle y == 0.
    if y == 0.0 {
        if x > 0.0 || (x.to_bits() == 0x0000_0000_0000_0000) {
            return y; // ±0
        }
        // x is negative or -0.0
        if y.to_bits() & 0x8000_0000_0000_0000 != 0 {
            return -PI;
        }
        return PI;
    }

    // Handle x == 0.
    if x == 0.0 {
        return if y > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
    }

    // Handle infinities.
    let x_inf: bool = x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF == 0x7FF0_0000_0000_0000;
    let y_inf: bool = y.to_bits() & 0x7FFF_FFFF_FFFF_FFFF == 0x7FF0_0000_0000_0000;

    if y_inf && x_inf {
        let qtr: f64 = PI / 4.0;
        if x > 0.0 {
            return if y > 0.0 { qtr } else { -qtr };
        }
        let three_qtr: f64 = 3.0 * PI / 4.0;
        return if y > 0.0 { three_qtr } else { -three_qtr };
    }
    if y_inf {
        return if y > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
    }
    if x_inf {
        if x > 0.0 {
            return if y > 0.0 { 0.0 } else { -0.0 };
        }
        return if y > 0.0 { PI } else { -PI };
    }

    // General case.
    let a: f64 = crate::atan::atan(y / x);
    if x > 0.0 {
        a
    } else if y >= 0.0 {
        a + PI
    } else {
        a - PI
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_first_quadrant() {
        assert!((atan2(1.0, 1.0) - PI / 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_second_quadrant() {
        assert!((atan2(1.0, -1.0) - 3.0 * PI / 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_third_quadrant() {
        assert!((atan2(-1.0, -1.0) - (-3.0 * PI / 4.0)).abs() < 1e-10);
    }

    #[test]
    fn test_axes() {
        assert!((atan2(1.0, 0.0) - FRAC_PI_2).abs() < 1e-10);
        assert!((atan2(-1.0, 0.0) - (-FRAC_PI_2)).abs() < 1e-10);
        assert!((atan2(0.0, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(atan2(f64::NAN, 1.0).is_nan());
    }
}
