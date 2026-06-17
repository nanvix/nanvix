// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Rounds a floating-point number to the nearest integer, away from zero.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The rounded value of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn round(x: f64) -> f64 {
    let t: f64 = crate::trunc::trunc(x);
    let d: f64 = crate::fabs::fabs(x - t);
    if d >= 0.5 {
        if x > 0.0 {
            t + 1.0
        } else {
            t - 1.0
        }
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
    fn test_round_up() {
        assert!((round(2.5) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_round_down() {
        assert!((round(2.3) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_round() {
        assert!((round(-2.5) - (-3.0)).abs() < 1e-10);
        assert!((round(-2.3) - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(round(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(round(f64::INFINITY), f64::INFINITY);
    }
}
