// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd::syscall,
};
use ::alloc::{
    ffi::CString,
    vec::Vec,
};
use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the current working directory pathname. The `getcwd()` function copies the absolute
/// pathname of the current working directory into the buffer pointed to by `buf`, which is `size`
/// bytes long. The pathname is null-terminated and contains no more than `size` bytes, including
/// the null terminator. This function provides a way to obtain the current working directory path
/// for use in relative path resolution and directory navigation operations.  The returned path is
/// always an absolute path starting with `/` and represents the full path from the root directory
/// to the current working directory.
///
/// # Parameters
///
/// - `buf`: Pointer to a buffer where the current working directory pathname will be stored.
///   This must be a valid pointer to a writable memory area of at least `size` bytes. The
///   buffer will receive the null-terminated absolute pathname of the current working directory.
///   The buffer must remain valid and writable for the duration of the function call.
/// - `size`: Size of the buffer in bytes, including space for the null terminator. This value
///   must be greater than `0` and large enough to hold the complete pathname plus the null
///   terminator. If the buffer is too small to hold the complete pathname, the function will
///   fail with an error.
///
/// # Returns
///
/// The `getcwd()` function returns a pointer to `buf` on success, containing the null-terminated
/// absolute pathname of the current working directory. On error, it returns `NULL` and sets
/// `errno` to indicate the error. Common error conditions include buffer too small to hold
/// the pathname, invalid buffer pointer, buffer size is zero, or the current working directory
/// is inaccessible or has been removed.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buf` points to a valid, writable memory location of at least `size` bytes.
/// - `buf` remains valid and writable for the duration of the function call.
/// - `buf` is properly aligned for byte access.
/// - `size` is greater than `0` and accurately represents the size of the buffer.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: c_size_t) -> *mut c_char {
    // Check if the buffer is valid.
    if buf.is_null() {
        ::syslog::error!("getcwd(): invalid buffer (buf={buf:?}, size={size:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return core::ptr::null_mut();
    }

    // Check if the buffer size is invalid.
    if size == 0 {
        ::syslog::error!("getcwd(): buffer size is zero (buf={buf:?}, size={size:?})");
        *__errno_location() = ErrorCode::ValueOutOfRange.get();
        return core::ptr::null_mut();
    }

    // Get current working directory and check for errors.
    match syscall::getcwd() {
        // Success.
        Ok(cwd) => {
            // Copy current working directory to the buffer.
            let cstr: CString = match CString::new(cwd) {
                Ok(cstr) => cstr,
                Err(_) => {
                    ::syslog::error!(
                        "getcwd(): invalid current working directory (buf={buf:?}, size={size:?})"
                    );
                    *__errno_location() = ErrorCode::InvalidArgument.get();
                    return core::ptr::null_mut();
                },
            };

            let cwd: Vec<u8> = cstr.into_bytes_with_nul();
            let buf: &mut [u8] = slice::from_raw_parts_mut(buf as *mut u8, size as usize);

            // Check if the buffer is large enough.
            if buf.len() < cwd.len() {
                ::syslog::error!("getcwd(): buffer is too small (buf={buf:?}, size={size:?})");
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
                return core::ptr::null_mut();
            }

            buf[..cwd.len()].copy_from_slice(&cwd);

            // Return the buffer.
            buf.as_mut_ptr() as *mut c_char
        },
        // Failure.
        Err(error) => {
            ::syslog::error!("getcwd(): {error:?} (buf={buf:?}, size={size:?})");
            *__errno_location() = error.code.get();
            core::ptr::null_mut()
        },
    }
}
