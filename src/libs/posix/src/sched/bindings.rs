// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Yields the processor.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, it returns `-1` and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::sched::sched_yield()`]
///
#[unsafe(no_mangle)]
pub extern "C" fn sched_yield() -> c_int {
    match crate::sched::sched_yield() {
        Ok(_) => 0,
        Err(e) => {
            // System call failed. Set errno.
            unsafe { *__errno_location() = e.code.get() };
            -1
        },
    }
}
