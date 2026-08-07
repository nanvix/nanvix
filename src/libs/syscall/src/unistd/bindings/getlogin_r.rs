// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Constants
//==================================================================================================

/// Login name reported for the single-user Nanvix system, including its NUL terminator.
const LOGIN_NAME: &[u8] = b"root\0";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stores the name of the user associated with the controlling terminal of the calling process into
/// the buffer pointed to by `buf`. Nanvix is a single-user system, so the login name is always
/// `"root"`.
///
/// # Parameters
///
/// - `buf`: Destination buffer that receives the NUL-terminated login name.
/// - `bufsize`: Size of `buf`, in bytes.
///
/// # Returns
///
/// Upon successful completion, `getlogin_r()` returns `0`. If `buf` is `NULL`, `EINVAL` is returned.
/// If `bufsize` is too small to hold the login name and its terminator, `ERANGE` is returned. Unlike
/// most interfaces, the error number is returned directly rather than through `errno`.
///
/// # Safety
///
/// The caller must ensure that `buf` points to a writable region of at least `bufsize` bytes.
///
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getlogin_r(buf: *mut c_char, bufsize: c_size_t) -> c_int {
    if buf.is_null() {
        return ErrorCode::InvalidArgument.get();
    }

    let bufsize: usize = bufsize as usize;
    if bufsize < LOGIN_NAME.len() {
        return ErrorCode::ValueOutOfRange.get();
    }

    // SAFETY: `buf` is non-null and `bufsize >= LOGIN_NAME.len()`, so the copy stays within bounds.
    // The source and destination do not overlap.
    unsafe {
        ::core::ptr::copy_nonoverlapping(LOGIN_NAME.as_ptr(), buf.cast::<u8>(), LOGIN_NAME.len());
    }

    0
}
