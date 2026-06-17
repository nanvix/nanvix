// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the square root of `x`.
///
/// # Description
///
/// Lowers to the target's hardware square-root instruction (`fsqrt` / `sqrtsd`) via the
/// `sqrtf64` intrinsic, which is fully IEEE-754 correct (including NaN, infinity, and signed
/// zero) and matches the newlib implementation's performance.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The square root of `x`. Returns NaN for negative inputs.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sqrt(x: f64) -> f64 {
    core::intrinsics::sqrtf64(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_squares() {
        assert!((sqrt(4.0) - 2.0).abs() < 1e-10);
        assert!((sqrt(9.0) - 3.0).abs() < 1e-10);
        assert!((sqrt(16.0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_non_perfect() {
        assert!((sqrt(2.0) - 1.41421356237).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(sqrt(0.0), 0.0);
    }

    #[test]
    fn test_one() {
        assert!((sqrt(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!(sqrt(-1.0).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(sqrt(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(sqrt(f64::INFINITY), f64::INFINITY);
    }
}
