// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the smallest integer value not less than `x`.
///
/// # Description
///
/// Returns the ceiling of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The smallest integer not less than `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ceil(x: f64) -> f64 {
    let t: f64 = crate::trunc::trunc(x);
    if x > t {
        t + 1.0
    } else {
        t
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
