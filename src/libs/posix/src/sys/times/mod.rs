// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::syscall::sys::times;
use ::syslog::trace_syscall;
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
/// past. Otherwise, it returns `-1`` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is not safe because:
/// - It dereferences a raw pointer.
/// - It may access global variables.
///
/// It is safe to call this function if the following conditions are met:
/// - `buffer` points to a valid `tms` structure.
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn times(buffer: *mut tms) -> clock_t {
    // Convert `buffer` pointer to a reference.
    // NOTE: We provide same semantics of Linux: `buffer` can be a null pointer.
    let mut buffer: Option<&mut tms> = if buffer.is_null() {
        None
    } else {
        Some(&mut *buffer)
    };

    // Get process times and parse the result.
    match times::times(&mut buffer) {
        // System call succeeded.
        Ok(clock) => clock,
        // System call failed.
        Err(error) => {
            ::syslog::error!("times(): failed (buffer={:?}, error={:?})", buffer, error);
            *__errno_location() = error.code.get();
            -1 as clock_t
        },
    }
}
