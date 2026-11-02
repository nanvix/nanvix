// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the smallest integer value not less than `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The smallest integer not less than `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ceilf(x: f32) -> f32 {
    let t: f32 = crate::truncf::truncf(x);
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
