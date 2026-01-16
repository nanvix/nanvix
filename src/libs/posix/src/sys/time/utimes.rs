// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::stat,
};
use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    ffi::{
        c_char,
        c_int,
    },
    sys_select::timeval,
    time::timespec,
};
use ::syslog::trace_syscall;

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
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn utimes(filename: *const c_char, times: *const timeval) -> c_int {
    // Check if `times` is invalid.
    if times.is_null() {
        ::syslog::error!("utimes(): invalid times (filename={:?}, times={:?})", filename, times);
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `times`.
    let times: &[timeval; 2] = match slice::from_raw_parts(times, 2).try_into() {
        Ok(times) => times,
        Err(_) => {
            ::syslog::error!(
                "utimes(): invalid times (filename={:?}, times={:?})",
                filename,
                times
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `times[0]` to `timespec`.
    let times_0: timespec = match times[0].try_into() {
        Ok(timespec) => timespec,
        Err(error) => {
            ::syslog::error!(
                "utimes(): failed to convert times[0] (filename={filename:?}, times={times:?}, \
                 error={error:?})",
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `times[1]` to `timespec`.
    let times_1: timespec = match times[1].try_into() {
        Ok(timespec) => timespec,
        Err(error) => {
            ::syslog::error!(
                "utimes(): failed to convert times[1] (filename={filename:?}, times={times:?}, \
                 error={error:?})",
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let times: [timespec; 2] = [times_0, times_1];

    stat::utimensat(AT_FDCWD, filename, times.as_ptr(), 0)
}
