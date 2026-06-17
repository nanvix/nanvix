// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes `e^x - 1`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `expm1(x) = e^x - 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn expm1(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    crate::exp::exp(x) - 1.0
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert_eq!(expm1(0.0), 0.0);
    }

    #[test]
    fn test_one() {
        assert!((expm1(1.0) - 1.718_281_828_459_045).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((expm1(-1.0) + 0.632_120_558_828_557_7).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(expm1(f64::NAN).is_nan());
    }
}
