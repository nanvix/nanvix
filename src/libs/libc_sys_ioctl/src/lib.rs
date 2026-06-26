// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Standalone Functions
//==================================================================================================

mod bindings {
    use ::sysapi::ffi::{
        c_int,
        c_ulong,
        c_void,
    };
    use ::syscall::errno::__errno_location;
    use ::syslog::trace_syscall;

    ///
    /// # Description
    ///
    /// Performs a control operation on the device referred to by `fd`. Terminal-control requests
    /// (`TCGETS`/`TCSETS`/`TIOCGWINSZ`/`TIOCSWINSZ`) on a console descriptor are routed to the vfsd
    /// console backend; every other request returns `0` without acting.
    ///
    /// # Parameters
    ///
    /// - `fd`: File descriptor.
    /// - `request`: Device-dependent request code.
    /// - `arg`: Pointer to the request's argument (a `struct termios` or `struct winsize` for the
    ///   terminal requests).
    ///
    /// # Returns
    ///
    /// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer supplied by a foreign caller.
    /// It is safe to call when, for a terminal-control request, `arg` points to a valid object of
    /// the type the request expects.
    ///
    #[unsafe(no_mangle)]
    #[trace_syscall]
    pub unsafe extern "C" fn ioctl(fd: c_int, request: c_ulong, arg: *mut c_void) -> c_int {
        match unsafe { ::syscall::sys::ioctl::ioctl(fd, request as c_int, arg) } {
            Ok(ret) => ret,
            Err(error) => {
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
}
