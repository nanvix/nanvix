// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const FRAC_PI_2: f32 = core::f32::consts::FRAC_PI_2;
const FRAC_PI_4: f32 = core::f32::consts::FRAC_PI_4;

//==================================================================================================
// Internal Helpers
//==================================================================================================

fn atan_poly(x: f32) -> f32 {
    let x2: f32 = x * x;
    x * (1.0 + x2 * (-1.0 / 3.0 + x2 * (1.0 / 5.0 + x2 * (-1.0 / 7.0 + x2 * (1.0 / 9.0)))))
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the arctangent of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The arctangent of `x` in radians.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn atanf(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x.to_bits() == 0x7F80_0000 {
        return FRAC_PI_2;
    }
    if x.to_bits() == 0xFF80_0000 {
        return -FRAC_PI_2;
    }

    let abs_x: f32 = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    let sign: f32 = if x < 0.0 { -1.0 } else { 1.0 };

    if abs_x > 1.0 {
        sign * (FRAC_PI_2 - atan_poly(1.0 / abs_x))
    } else if abs_x > 0.414_213_6 {
        let t: f32 = (abs_x - 1.0) / (abs_x + 1.0);
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
        assert!((atanf(0.0)).abs() < 1e-5);
    }

    #[test]
    fn test_one() {
        assert!((atanf(1.0) - FRAC_PI_4).abs() < 1e-4);
    }

    #[test]
    fn test_nan() {
        assert!(atanf(f32::NAN).is_nan());
    }
}
