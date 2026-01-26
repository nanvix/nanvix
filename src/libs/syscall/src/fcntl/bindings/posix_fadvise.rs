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
/// Provides advice about the use of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
/// - `advice`: Advice to provide.
///
/// # Returns
///
/// Upon success, `posix_fadvise()` returns `0`. Otherwise, it returns an error code to indicate the
/// error.
///
/// # Safety
///
/// This function is unsafe because it may access global variables.
///
/// It is safe to call this function if the following conditions are met:
/// - This function is not called from multiple threads at the same time.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_fadvise(
    fd: c_int,
    offset: off_t,
    len: off_t,
    advice: c_int,
) -> c_int {
    // Run system call and check for errors.
    match fcntl::posix_fadvise(fd, offset, len, advice) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "posix_fadvise(): failed (fd={fd:?}, offset={offset:?}, len={len:?}, \
                 advice={advice:?})"
            );

            error.code.get()
        },
    }
}
