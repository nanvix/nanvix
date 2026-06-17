// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the largest integer value not greater than `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The largest integer not greater than `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn floor(x: f64) -> f64 {
    let t: f64 = crate::trunc::trunc(x);
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
        assert!((floor(2.7) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_fraction() {
        assert!((floor(-2.3) - (-3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_integer() {
        assert!((floor(5.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(floor(0.0).to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn test_nan() {
        assert!(floor(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(floor(f64::INFINITY), f64::INFINITY);
        assert_eq!(floor(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }
}
