// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fcntl;
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Ensures that the file space is allocated for a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
///
/// # Returns
///
/// Upon successful completion, `posix_fallocate()` returns `0`. Otherwise, it returns an error code
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may access global variables.
///
/// It is safe to call this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[trace_libcall]
#[no_mangle]
pub unsafe extern "C" fn posix_fallocate(fd: c_int, offset: off_t, len: off_t) -> c_int {
    // Run system call and check for errors.
    match fcntl::posix_fallocate(fd, offset, len) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "posix_fallocate(): {error:?} (fd={fd:?}, offset={offset:?}, len={len:?})",
            );
            error.code.get()
        },
    }
}
