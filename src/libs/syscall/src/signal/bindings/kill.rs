// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    number::KcallNumber,
};
use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
    sys_types::pid_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a signal to a process.
///
/// # Parameters
///
/// - `pid`: Target process identifier.
/// - `signal`: Signal number to send.
///
/// # Returns
///
/// `0` on success, `-1` on error (with errno set).
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub extern "C" fn kill(pid: pid_t, signal: c_int) -> c_int {
    let result: i64 = ::sys::kcall2!(
        KcallNumber::Kill.into(),
        pid as u32,
        signal as u32
    );

    if result < 0 {
        unsafe {
            *__errno_location() = match ErrorCode::try_from(result) {
                Ok(code) => code.get(),
                Err(_) => ErrorCode::InvalidSysCall.get(),
            };
        }
        return -1;
    }

    0
}
