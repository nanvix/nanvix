// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LOG2_E: f32 = core::f32::consts::LOG2_E;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the base-2 logarithm of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value (must be positive).
///
/// # Returns
///
/// The base-2 logarithm of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn log2f(x: f32) -> f32 {
    crate::logf::logf(x) * LOG2_E
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_two() {
        assert!((log2f(2.0) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_one() {
        assert!((log2f(1.0)).abs() < 1e-4);
    }
}
