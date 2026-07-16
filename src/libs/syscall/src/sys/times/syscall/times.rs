// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use sysapi::{
    sys_times::tms,
    sys_types::clock_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the current process times.
///
/// # Parameters
///
/// - `buffer`: Buffer to store the times.
///
/// # Returns
///
/// Upon successful completion, `times()` returns the elapsed time since an arbitrary point in the
/// past. Otherwise, an error code is returned.
///
pub fn times(buffer: &mut Option<&mut tms>) -> Result<clock_t, Error> {
    ::syslog::trace!("times(): {:?}", buffer);
    if let Some(buf) = buffer {
        buf.tms_utime = 0;
        buf.tms_stime = 0;
        buf.tms_cutime = 0;
        buf.tms_cstime = 0;
    }
    Ok(0)
}
