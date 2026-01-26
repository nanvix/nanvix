// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use ::sysapi::ffi::c_int;
    use ::syslog::trace_syscall;

    #[allow(clippy::missing_safety_doc)]
    #[unsafe(no_mangle)]
    #[trace_syscall]
    pub extern "C" fn ioctl(_fd: c_int, _request: c_int, _arg: *mut c_int) -> c_int {
        // TODO: https://github.com/nanvix/nanvix/issues/351
        ::syslog::debug!("ioctl(): not implemented, ignoring");
        0
    }
}
