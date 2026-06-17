// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LOG10_E: f64 = core::f64::consts::LOG10_E;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the base-10 logarithm of `x`.
///
/// # Parameters
///
/// - `x`: Input value (must be positive).
///
/// # Returns
///
/// The base-10 logarithm of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn log10(x: f64) -> f64 {
    crate::log::log(x) * LOG10_E
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((log10(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_ten() {
        assert!((log10(10.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_hundred() {
        assert!((log10(100.0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_zero() {
        assert_eq!(log10(0.0), f64::NEG_INFINITY);
    }
}
