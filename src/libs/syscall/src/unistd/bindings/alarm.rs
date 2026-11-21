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
/// Replaces any previous alarm. A zero delay cancels the timer without clearing an already pending
/// signal. Alarms are not inherited by `fork()` and survive `exec()`.
///
/// # Parameters
///
/// - `seconds`: The requested delay, in seconds, or zero to cancel.
///
/// # Returns
///
/// Returns the remaining time on the previous alarm, rounded up to whole seconds, or zero if none
/// was scheduled. An unexpected kernel error is logged and returns `c_uint::MAX`; POSIX reserves no
/// error value for this function and `errno` is left unchanged.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub extern "C" fn alarm(seconds: c_uint) -> c_uint {
    match ::sys::kcall::pm::__kcall_alarm(seconds) {
        Ok(remaining) => remaining,
        Err(error) => {
            ::syslog::warn!("alarm(): failed (seconds={seconds}, error={error:?})");
            c_uint::MAX
        },
    }
}
