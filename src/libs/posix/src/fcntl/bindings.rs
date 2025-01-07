// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
        c_char,
        c_int,
    },
    sys::types::mode_t,
};
use ::core::ffi;
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens the file specified by `pathname`.
///
/// # Parameters
///
/// - `path`:  Pathname of the file to open.
/// - `flags`: Flags to open the file.
/// - `mode`:  Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the `open()` system call returns a non-negative integer representing
/// the lowest numbered unused file descriptor. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::fcntl::open()`]
///
#[no_mangle]
pub extern "C" fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match unsafe { ffi::CStr::from_ptr(path).to_str() } {
        Ok(pathname) => pathname,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    let retcode: c_int = crate::fcntl::open(pathname, flags, mode);

    // Check if the system call failed.
    if retcode < 0 {
        unsafe {
            errno = match ErrorCode::try_from(retcode) {
                Ok(e) => e.into_errno(),
                Err(_) => {
                    ::nvx::log!("open(): invalid error code");
                    ErrorCode::ValueOutOfRange.into_errno()
                },
            };
        }
        return -1;
    }

    0
}
