// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use crate::errno::__errno_location;
    use ::sys::error::ErrorCode;
    use ::sysapi::{
        ffi::c_int,
        pwd::passwd,
    };
    use ::syslog::trace_libcall;

    #[allow(clippy::missing_safety_doc)]
    #[unsafe(no_mangle)]
    #[trace_libcall]
    pub unsafe extern "C" fn getpwuid(_uid: c_int) -> *mut passwd {
        ::syslog::debug!("getpwuid(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        core::ptr::null_mut()
    }
}
