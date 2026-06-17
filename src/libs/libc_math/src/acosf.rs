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

/// Computes the arccosine of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value in `[-1, 1]`.
///
/// # Returns
///
/// The arccosine of `x` in radians. NaN if `|x| > 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn acosf(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    let abs_x: f32 = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    if abs_x > 1.0 {
        return f32::from_bits(0x7FC0_0000);
    }
    if x == 1.0 {
        return 0.0;
    }
    if x == -1.0 {
        return PI;
    }
    FRAC_PI_2 - crate::asinf::asinf(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((acosf(1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_zero() {
        assert!((acosf(0.0) - FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn test_out_of_range() {
        assert!(acosf(1.5).is_nan());
    }
}
