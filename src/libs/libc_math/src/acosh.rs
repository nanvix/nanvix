// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the inverse hyperbolic cosine of `x`.
///
/// # Parameters
///
/// - `x`: Input value (must be `>= 1`).
///
/// # Returns
///
/// The value `acosh(x) = ln(x + sqrt(x^2 - 1))`, or NaN for `x < 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn acosh(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < 1.0 {
        return f64::NAN;
    }
    crate::log::log(x + crate::sqrt::sqrt(x * x - 1.0))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
