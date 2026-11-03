// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the square root of `x`.
///
/// # Description
///
/// Lowers to the target's hardware square-root instruction (`fsqrt` / `sqrtsd`) via the
/// `sqrtf64` intrinsic, which is fully IEEE-754 correct (including NaN, infinity, and signed
/// zero) and matches the newlib implementation's performance.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The square root of `x`. Returns NaN for negative inputs.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sqrt(x: f64) -> f64 {
    core::intrinsics::sqrtf64(x)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
