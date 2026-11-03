// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the inverse hyperbolic tangent of `x`.
///
/// # Parameters
///
/// - `x`: Input value (must satisfy `|x| <= 1`).
///
/// # Returns
///
/// The value `atanh(x) = 0.5 * ln((1 + x) / (1 - x))`, or NaN for `|x| > 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn atanh(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x == 1.0 {
        return f64::INFINITY;
    }
    if x == -1.0 {
        return f64::NEG_INFINITY;
    }
    if x.abs() > 1.0 {
        return f64::NAN;
    }
    0.5 * crate::log::log((1.0 + x) / (1.0 - x))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
