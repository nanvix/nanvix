// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const FRAC_PI_2: f32 = core::f32::consts::FRAC_PI_2;
const FRAC_2_PI: f32 = core::f32::consts::FRAC_2_PI;

//==================================================================================================
// Internal Helpers
//==================================================================================================

fn sin_kernel(x: f32) -> f32 {
    let x2: f32 = x * x;
    x * (1.0
        + x2 * (-1.666_666_7e-1 + x2 * (8.333_334e-3 + x2 * (-1.984_127e-4 + x2 * 2.755_732e-6))))
}

fn cos_kernel(x: f32) -> f32 {
    let x2: f32 = x * x;
    1.0 + x2 * (-0.5 + x2 * (4.166_666_7e-2 + x2 * (-1.388_888_9e-3 + x2 * 2.480_158_8e-5)))
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the sine of `x` in radians (single-precision).
///
/// # Parameters
///
/// - `x`: Input angle in radians.
///
/// # Returns
///
/// The sine of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sinf(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x.to_bits() & 0x7F80_0000 == 0x7F80_0000 {
        return f32::from_bits(0x7FC0_0000);
    }

    let k_f: f32 = crate::roundf::roundf(x * FRAC_2_PI);

    #[allow(clippy::cast_possible_truncation)]
    let k: i64 = k_f as i64;

    let r: f32 = x - k_f * FRAC_PI_2;
    let quadrant: u64 = (k as u64) & 3;

    match quadrant {
        0 => sin_kernel(r),
        1 => cos_kernel(r),
        2 => -sin_kernel(r),
        3 => -cos_kernel(r),
        _ => sin_kernel(r),
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
        assert!((sinf(0.0)).abs() < 1e-5);
    }

    #[test]
    fn test_pi_over_2() {
        assert!((sinf(FRAC_PI_2) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_pi_over_6() {
        assert!((sinf(core::f32::consts::PI / 6.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_nan() {
        assert!(sinf(f32::NAN).is_nan());
    }
}
