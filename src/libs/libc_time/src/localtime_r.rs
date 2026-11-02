// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::tm;
use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a calendar time (`time_t`) to a broken-down local time representation.
///
/// In Nanvix (no timezone support), this function is equivalent to [`gmtime_r`].
///
/// # Parameters
///
/// - `timep`: Pointer to the calendar time to convert.
/// - `result`: Pointer to a `tm` structure where the broken-down time will be stored.
///
/// # Returns
///
/// On success, returns `result`. Returns a null pointer if `timep` or `result` is null.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `timep` and `result`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn localtime_r(timep: *const time_t, result: *mut tm) -> *mut tm {
    crate::gmtime_r::gmtime_r(timep, result)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
