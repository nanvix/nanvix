// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the hyperbolic cosine of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `cosh(x) = (e^x + e^-x) / 2`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn cosh(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    let e: f64 = crate::exp::exp(x.abs());
    (e + 1.0 / e) * 0.5
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
