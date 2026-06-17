// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the smallest integer value not less than `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The smallest integer not less than `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ceilf(x: f32) -> f32 {
    let t: f32 = crate::truncf::truncf(x);
    if x > t {
        t + 1.0
    } else {
        t
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive_fraction() {
        assert!((ceilf(2.3) - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_negative_fraction() {
        assert!((ceilf(-2.3) - (-2.0)).abs() < 1e-5);
    }

    #[test]
    fn test_nan() {
        assert!(ceilf(f32::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(ceilf(f32::INFINITY), f32::INFINITY);
    }
}
