// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the absolute value of a floating-point number.
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
pub extern "C" fn fabs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive() {
        let result: f64 = fabs(3.14);
        assert!((result - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        let result: f64 = fabs(-2.72);
        assert!((result - 2.72).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(fabs(0.0).to_bits(), 0u64);
        assert_eq!(fabs(-0.0).to_bits(), 0u64);
    }

    #[test]
    fn test_nan() {
        assert!(fabs(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(fabs(f64::INFINITY), f64::INFINITY);
        assert_eq!(fabs(f64::NEG_INFINITY), f64::INFINITY);
    }
}
