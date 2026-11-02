// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the hyperbolic sine of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `sinh(x) = (e^x - e^-x) / 2`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sinh(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        return x;
    }
    let e: f64 = crate::exp::exp(x);
    (e - 1.0 / e) * 0.5
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
