// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the complementary error function of `x`, `erfc(x) = 1 - erf(x)`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `erfc(x)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn erfc(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    1.0 - crate::erf::erf(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((erfc(0.0) - 1.0).abs() < 1e-7);
    }

    #[test]
    fn test_one() {
        assert!((erfc(1.0) - 0.157_299_207_050_285_1).abs() < 1e-6);
    }

    #[test]
    fn test_large() {
        assert!((erfc(5.0)).abs() < 1e-6);
    }

    #[test]
    fn test_nan() {
        assert!(erfc(f64::NAN).is_nan());
    }
}
