// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the file offset of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset to set.
/// - `whence`: Reference point for the offset.
///
/// # Returns
///
/// Upon successful completion, `lseek()` returns the resulting offset. Otherwise, it returns
/// `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because it may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    // Attempt to seek the file descriptor and check for errors.
    match unistd::lseek(fd, offset, whence) {
        Ok(offset) => offset,
        Err(error) => {
            ::syslog::trace!(
                "lseek(): {error:?} (fd={fd:?}, offset={offset:?}, whence={whence:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
