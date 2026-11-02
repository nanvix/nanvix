// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the square root of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value (must be non-negative).
///
/// # Returns
///
/// The square root of `x`. Returns NaN for negative inputs.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sqrtf(x: f32) -> f32 {
    core::intrinsics::sqrtf32(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
