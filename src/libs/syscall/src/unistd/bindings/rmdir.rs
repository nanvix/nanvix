// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::atflags::{
        AT_FDCWD,
        AT_REMOVEDIR,
    },
    ffi::{
        c_char,
        c_int,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn rmdir(path: *const c_char) -> c_int {
    // Validate the path pointer.
    if path.is_null() {
        ::syslog::warn!("rmdir(): path is null (path={path:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Convert C string to Rust string.
    let pathname: &str = match core::ffi::CStr::from_ptr(path).to_str() {
        Ok(p) => p,
        Err(_) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Route through unlinkat with AT_REMOVEDIR.
    match crate::fcntl::syscall::unlinkat(AT_FDCWD, pathname, AT_REMOVEDIR) {
        Ok(()) => 0,
        Err(e) => {
            ::syslog::warn!("rmdir(): failed (path={pathname:?}, error={e:?})");
            *__errno_location() = e.code.get();
            -1
        },
    }
}
