// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the largest integer value not greater than `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The largest integer not greater than `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn floorf(x: f32) -> f32 {
    let t: f32 = crate::truncf::truncf(x);
    if x < t {
        t - 1.0
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
        assert!((floorf(2.7) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_negative_fraction() {
        assert!((floorf(-2.3) - (-3.0)).abs() < 1e-5);
    }

    #[test]
    fn test_nan() {
        assert!(floorf(f32::NAN).is_nan());
    }
}
