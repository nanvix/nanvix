// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the square root of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value (must be non-negative).
///
/// # Returns
///
/// The square root of `x`. Returns NaN for negative inputs.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sqrtf(x: f32) -> f32 {
    core::intrinsics::sqrtf32(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_squares() {
        assert!((sqrtf(4.0) - 2.0).abs() < 1e-5);
        assert!((sqrtf(9.0) - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_non_perfect() {
        assert!((sqrtf(2.0) - 1.41421).abs() < 1e-4);
    }

    #[test]
    fn test_negative() {
        assert!(sqrtf(-1.0).is_nan());
    }

    #[test]
    fn test_zero() {
        assert_eq!(sqrtf(0.0), 0.0);
    }
}
