// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Returns the minimum of two floating-point values (NaN-aware).
///
/// # Description
///
/// If one argument is NaN, the other is returned. If both are NaN, NaN is returned.
///
/// # Parameters
///
/// - `x`: First value.
/// - `y`: Second value.
///
/// # Returns
///
/// The minimum of `x` and `y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fmin(x: f64, y: f64) -> f64 {
    if x.is_nan() {
        return y;
    }
    if y.is_nan() {
        return x;
    }
    // Handle signed zeros (and any mixed signs): -0.0 compares less than +0.0,
    // so return the negatively-signed operand when the signs differ (C Annex
    // F.10.9.2).
    if x.is_sign_negative() != y.is_sign_negative() {
        return if x.is_sign_negative() { x } else { y };
    }
    if x < y {
        x
    } else {
        y
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert!((fmin(2.0, 3.0) - 2.0).abs() < 1e-10);
        assert!((fmin(3.0, 2.0) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!((fmin(f64::NAN, 2.0) - 2.0).abs() < 1e-10);
        assert!((fmin(2.0, f64::NAN) - 2.0).abs() < 1e-10);
        assert!(fmin(f64::NAN, f64::NAN).is_nan());
    }

    #[test]
    fn test_negative() {
        assert!((fmin(-1.0, -2.0) - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_signed_zero() {
        // -0.0 is treated as smaller than +0.0, regardless of argument order.
        assert!(fmin(-0.0, 0.0).is_sign_negative());
        assert!(fmin(0.0, -0.0).is_sign_negative());
        assert_eq!(fmin(-0.0, 0.0), 0.0);
    }
}
