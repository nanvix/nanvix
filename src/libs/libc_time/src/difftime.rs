// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the difference between two calendar times as a floating-point number of seconds.
///
/// # Parameters
///
/// - `time1`: The later time value.
/// - `time0`: The earlier time value.
///
/// # Returns
///
/// The number of seconds elapsed from `time0` to `time1`, as a `f64`.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[allow(clippy::cast_precision_loss)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn difftime(time1: time_t, time0: time_t) -> f64 {
    // Compute the difference in floating point to avoid overflowing `time_t` for extreme inputs.
    (time1 as f64) - (time0 as f64)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
