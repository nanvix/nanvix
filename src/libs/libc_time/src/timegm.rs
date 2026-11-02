// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mktime::mktime,
    tm_struct::tm,
};
use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a broken-down time structure, interpreted as Coordinated Universal Time (UTC), to a
/// calendar time (`time_t`). The `tm` structure is normalized so that its fields are within their
/// proper ranges.
///
/// Nanvix keeps all time in UTC and applies no timezone offset, so `timegm` is equivalent to
/// [`mktime`].
///
/// # Parameters
///
/// - `timeptr`: Pointer to the broken-down time to convert.
///
/// # Returns
///
/// On success, the calendar time in seconds since the epoch. On error, `(time_t)(-1)`.
///
/// # Safety
///
/// This function is unsafe because it dereferences and writes to the raw pointer `timeptr`.
///
/// # References
///
/// - <https://www.man7.org/linux/man-pages/man3/timegm.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn timegm(timeptr: *mut tm) -> time_t {
    mktime(timeptr)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
