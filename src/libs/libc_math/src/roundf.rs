// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Rounds a single-precision floating-point number to the nearest integer, away from zero.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The rounded value of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn roundf(x: f32) -> f32 {
    let t: f32 = crate::truncf::truncf(x);
    let d: f32 = crate::fabsf::fabsf(x - t);
    if d >= 0.5 {
        if x > 0.0 {
            t + 1.0
        } else {
            t - 1.0
        }
    } else {
        t
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
