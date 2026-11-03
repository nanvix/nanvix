// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the largest integer value not greater than `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The largest integer not greater than `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn floor(x: f64) -> f64 {
    let t: f64 = crate::trunc::trunc(x);
    if x < t {
        t - 1.0
    } else {
        t
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
