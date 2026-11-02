// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the inverse hyperbolic sine of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `asinh(x) = sign(x) * ln(|x| + sqrt(x^2 + 1))`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn asinh(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() || x == 0.0 {
        return x;
    }
    let a: f64 = x.abs();
    let r: f64 = crate::log::log(a + crate::sqrt::sqrt(a * a + 1.0));
    if x < 0.0 {
        -r
    } else {
        r
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
