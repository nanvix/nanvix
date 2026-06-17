// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the natural logarithm of the absolute value of the gamma function (legacy `gamma`,
/// equivalent to [`lgamma`](crate::lgamma::lgamma)).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `ln(|Γ(x)|)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn gamma(x: f64) -> f64 {
    crate::lgamma::lgamma(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_alias_of_lgamma() {
        // gamma() is the legacy alias for lgamma().
        assert_eq!(gamma(3.0), crate::lgamma::lgamma(3.0));
        assert_eq!(gamma(5.0), crate::lgamma::lgamma(5.0));
    }

    #[test]
    fn test_nan() {
        assert!(gamma(f64::NAN).is_nan());
    }
}
