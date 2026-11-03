// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Multiplies a single-precision floating-point number by a power of two.
///
/// # Parameters
///
/// - `x`: Base value.
/// - `n`: Exponent.
///
/// # Returns
///
/// The value `x * 2^n`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn scalbnf(x: f32, n: c_int) -> f32 {
    crate::ldexpf::ldexpf(x, n)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
