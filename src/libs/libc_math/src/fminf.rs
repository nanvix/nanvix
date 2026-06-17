// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Returns the minimum of two single-precision values (NaN-aware).
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
pub extern "C" fn fminf(x: f32, y: f32) -> f32 {
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
        assert!((fminf(2.0, 3.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_nan() {
        assert!((fminf(f32::NAN, 2.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_signed_zero() {
        // -0.0 is treated as smaller than +0.0, regardless of argument order.
        assert!(fminf(-0.0, 0.0).is_sign_negative());
        assert!(fminf(0.0, -0.0).is_sign_negative());
    }
}
