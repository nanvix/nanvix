// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_uint;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Arranges for a `SIGALRM` signal to be delivered to the calling process after `seconds` seconds.
/// Nanvix does not yet support signal delivery, so no alarm is ever scheduled and this call has no
/// effect.
///
/// # Parameters
///
/// - `seconds`: The requested delay, in seconds. Ignored on Nanvix.
///
/// # Returns
///
/// Returns the number of seconds remaining on any previously scheduled alarm. As Nanvix never
/// schedules an alarm, this is always `0`.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub extern "C" fn alarm(seconds: c_uint) -> c_uint {
    // TODO: https://github.com/nanvix/nanvix/issues/453
    ::syslog::debug!("alarm(seconds={}): not implemented", seconds);
    0
}
