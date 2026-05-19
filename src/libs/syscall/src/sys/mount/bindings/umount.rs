// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unmounts the filesystem at the specified target path. The `umount()` function detaches the
/// filesystem mounted at the directory specified by `target`.
///
/// # Parameters
///
/// - `target`: Pointer to a null-terminated string specifying the mount point to unmount.
///
/// # Returns
///
/// The `umount()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `target` points to a valid null-terminated string.
/// - `target` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn umount(target: *const c_char) -> c_int {
    // Check if `target` is invalid.
    if target.is_null() {
        ::syslog::warn!("umount(): target is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `target`.
    let target_str: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::warn!("umount(): invalid target string");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to unmount and check for errors.
    match crate::sys::mount::umount(target_str) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!("umount(): {error:?} (target={target_str:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
