// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the tangent of `x` in radians (single-precision).
///
/// # Parameters
///
/// - `x`: Input angle in radians.
///
/// # Returns
///
/// The tangent of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tanf(x: f32) -> f32 {
    let s: f32 = crate::sinf::sinf(x);
    let c: f32 = crate::cosf::cosf(x);
    // Divide directly so IEEE-754 signed-zero semantics select the correct signed infinity at the
    // poles, where cosf(x) underflows to +0.0 or -0.0.
    s / c
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    const PI: f32 = core::f32::consts::PI;

    #[test]
    fn test_zero() {
        assert!((tanf(0.0)).abs() < 1e-5);
    }

    #[test]
    fn test_pi_over_4() {
        assert!((tanf(PI / 4.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_nan() {
        assert!(tanf(f32::NAN).is_nan());
    }
}
