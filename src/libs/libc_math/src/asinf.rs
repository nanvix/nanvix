// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const FRAC_PI_2: f32 = core::f32::consts::FRAC_PI_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the arcsine of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value in `[-1, 1]`.
///
/// # Returns
///
/// The arcsine of `x` in radians. NaN if `|x| > 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn asinf(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    let abs_x: f32 = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    if abs_x > 1.0 {
        return f32::from_bits(0x7FC0_0000);
    }
    if abs_x == 1.0 {
        return if x > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
    }

    if abs_x <= 0.5 {
        let denom: f32 = crate::sqrtf::sqrtf(1.0 - x * x);
        if denom == 0.0 {
            return if x > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
        }
        crate::atanf::atanf(x / denom)
    } else {
        let half: f32 = (1.0 - abs_x) * 0.5;
        let s: f32 = crate::sqrtf::sqrtf(half);
        let result: f32 =
            FRAC_PI_2 - 2.0 * crate::atanf::atanf(s / crate::sqrtf::sqrtf(1.0 - half));
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
        assert!((asinf(0.0)).abs() < 1e-5);
    }

    #[test]
    fn test_one() {
        assert!((asinf(1.0) - FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn test_out_of_range() {
        assert!(asinf(1.5).is_nan());
    }
}
