// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    fcntl::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
    },
    sys::time::timeval,
    time::timespec,
};
use ::core::slice;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets file access and modification times.
///
/// # Parameters
///
/// - `pathname`: Pathname of the file.
/// - `times`: Access and modification times.
///
/// # Returns
///
/// Upon successful completion, zero is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `filename` points to a valid null-terminated C string.
/// - `times` points to a valid array of length 2 of `timeval` structures.
///
#[no_mangle]
pub unsafe extern "C" fn utimes(filename: *const c_char, times: *const timeval) -> c_int {
    ::nvx::trace!("utimes(): filename={:?}, times={:?}", filename, times);

    // Check if `times` is invalid.
    if times.is_null() {
        ::nvx::error!("utimens(): invalid times (filename={:?}, times={:?})", filename, times);
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `times`.
    let times: &[timeval; 2] = match slice::from_raw_parts(times, 2).try_into() {
        Ok(times) => times,
        Err(_) => {
            ::nvx::error!("utimens(): invalid times (filename={:?}, times={:?})", filename, times);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };
    let times: [timespec; 2] = [timespec::from(times[0]), timespec::from(times[1])];

    crate::sys::stat::bindings::utimensat(AT_FDCWD, filename, times.as_ptr(), 0)
}
