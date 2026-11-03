// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the tangent of `x` in radians (single-precision).
///
/// # Parameters
///
/// - `x`: Input angle in radians.
///
/// # Returns
///
/// The tangent of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tanf(x: f32) -> f32 {
    let s: f32 = crate::sinf::sinf(x);
    let c: f32 = crate::cosf::cosf(x);
    // Divide directly so IEEE-754 signed-zero semantics select the correct signed infinity at the
    // poles, where cosf(x) underflows to +0.0 or -0.0.
    s / c
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
