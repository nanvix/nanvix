// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the absolute value of a single-precision floating-point number.
///
/// # Description
///
/// Returns the absolute value of `x` by clearing the sign bit.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The absolute value of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fabsf(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFF_FFFF)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive() {
        let result: f32 = fabsf(3.14);
        assert!((result - 3.14).abs() < 1e-5);
    }

    #[test]
    fn test_negative() {
        let result: f32 = fabsf(-2.72);
        assert!((result - 2.72).abs() < 1e-5);
    }

    #[test]
    fn test_zero() {
        assert_eq!(fabsf(0.0).to_bits(), 0u32);
        assert_eq!(fabsf(-0.0).to_bits(), 0u32);
    }

    #[test]
    fn test_nan() {
        assert!(fabsf(f32::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(fabsf(f32::INFINITY), f32::INFINITY);
        assert_eq!(fabsf(f32::NEG_INFINITY), f32::INFINITY);
    }
}
