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
mod tests;
