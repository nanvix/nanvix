// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Tests whether a single-precision value is NaN.
///
/// # Parameters
///
/// - `x`: Value to test.
///
/// # Returns
///
/// Non-zero if `x` is NaN, zero otherwise.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __isnanf(x: f32) -> c_int {
    if x.is_nan() {
        1
    } else {
        0
    }
}

/// Tests whether a double-precision value is NaN.
///
/// # Parameters
///
/// - `x`: Value to test.
///
/// # Returns
///
/// Non-zero if `x` is NaN, zero otherwise.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __isnand(x: f64) -> c_int {
    if x.is_nan() {
        1
    } else {
        0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
