// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const PI: f32 = core::f32::consts::PI;
const FRAC_PI_2: f32 = core::f32::consts::FRAC_PI_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the four-quadrant arctangent of `y / x` (single-precision).
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
pub extern "C" fn atan2f(y: f32, x: f32) -> f32 {
    if x.is_nan() || y.is_nan() {
        return f32::from_bits(0x7FC0_0000);
    }

    if y == 0.0 {
        if x > 0.0 || x.to_bits() == 0x0000_0000 {
            return y;
        }
        if y.to_bits() & 0x8000_0000 != 0 {
            return -PI;
        }
        return PI;
    }

    if x == 0.0 {
        return if y > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
    }

    let a: f32 = crate::atanf::atanf(y / x);
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
        assert!((atan2f(1.0, 1.0) - PI / 4.0).abs() < 1e-3);
    }

    #[test]
    fn test_axes() {
        assert!((atan2f(1.0, 0.0) - FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn test_nan() {
        assert!(atan2f(f32::NAN, 1.0).is_nan());
    }
}
