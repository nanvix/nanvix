// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `2` raised to the power `x` in single precision.
///
/// # Parameters
///
/// - `x`: Exponent.
///
/// # Returns
///
/// The value of `2^x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn exp2f(x: f32) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let result: f32 = crate::exp2::exp2(f64::from(x)) as f32;
    result
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_integer_powers() {
        assert_eq!(exp2f(0.0), 1.0);
        assert_eq!(exp2f(3.0), 8.0);
        assert_eq!(exp2f(-1.0), 0.5);
    }

    #[test]
    fn test_half() {
        assert!((exp2f(0.5) - core::f32::consts::SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn test_nan() {
        assert!(exp2f(f32::NAN).is_nan());
    }
}
