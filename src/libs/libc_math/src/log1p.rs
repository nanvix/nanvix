// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes `ln(1 + x)`.
///
/// # Parameters
///
/// - `x`: Input value (must satisfy `x >= -1`).
///
/// # Returns
///
/// The value `log1p(x) = ln(1 + x)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn log1p(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x == -1.0 {
        return f64::NEG_INFINITY;
    }
    if x < -1.0 {
        return f64::NAN;
    }
    crate::log::log(1.0 + x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert_eq!(log1p(0.0), 0.0);
    }

    #[test]
    fn test_one() {
        assert!((log1p(1.0) - core::f64::consts::LN_2).abs() < 1e-10);
    }

    #[test]
    fn test_negative_one() {
        assert_eq!(log1p(-1.0), f64::NEG_INFINITY);
    }

    #[test]
    fn test_below_minus_one() {
        assert!(log1p(-2.0).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(log1p(f64::NAN).is_nan());
    }
}
