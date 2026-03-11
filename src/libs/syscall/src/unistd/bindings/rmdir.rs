// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
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
        ::syslog::error!("rmdir(): path is null (path={path:?})");
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

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        match ::nvx::vfs::fd::vfs_rmdir(pathname) {
            Ok(()) => 0,
            Err(e) => {
                let code: ErrorCode = e.into();
                ::syslog::error!("rmdir(): VFS rmdir failed (path={pathname:?}, error={e})");
                *__errno_location() = code.get();
                -1
            },
        }
    }

    #[cfg(not(feature = "standalone"))]
    {
        // linuxd does not support rmdir — return ENOSYS for non-VFS paths.
        ::syslog::debug!("rmdir(): not supported for non-VFS path {:?}", pathname);
        *__errno_location() = ErrorCode::InvalidSysCall.get();
        -1
    }
}
