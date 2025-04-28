// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::{
        times::tms,
        types::clock_t,
    },
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
/// Upon successful completion, the `times()` system call returns the elapsed time since an
/// arbitrary point in the past. Otherwise, it returns -1 and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is not safe because it may dereferences raw pointers.
///
#[no_mangle]
pub unsafe extern "C" fn times(buffer: *mut tms) -> clock_t {
    let buffer: Option<&mut tms> = if buffer.is_null() {
        None
    } else {
        Some(&mut *buffer)
    };
    match crate::sys::times::times(buffer) {
        Ok(clock) => clock,
        Err(e) => {
            // Set errno.
            *__errno_location() = e.code.get();
            -1 as clock_t
        },
    }
}
