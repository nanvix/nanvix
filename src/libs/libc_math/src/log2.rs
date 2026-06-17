// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LOG2_E: f64 = core::f64::consts::LOG2_E;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the base-2 logarithm of `x`.
///
/// # Parameters
///
/// - `x`: Input value (must be positive).
///
/// # Returns
///
/// The base-2 logarithm of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn log2(x: f64) -> f64 {
    crate::log::log(x) * LOG2_E
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((log2(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_two() {
        assert!((log2(2.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_eight() {
        assert!((log2(8.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(log2(0.0), f64::NEG_INFINITY);
    }

    #[test]
    fn test_negative() {
        assert!(log2(-1.0).is_nan());
    }
}
