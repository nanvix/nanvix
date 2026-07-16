// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the foreground process group of the terminal referred to by `fd`. This is implemented
/// through the `TIOCSPGRP` ioctl, which the process manager daemon answers as the owner of the
/// controlling terminal's foreground group.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the controlling terminal.
/// - `pgrp`: The process-group ID to make the foreground group.
///
/// # Returns
///
/// `0` on success, or `-1` with `errno` set on failure.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn tcsetpgrp(fd: c_int, pgrp: pid_t) -> c_int {
    if pgrp < 0 {
        // SAFETY: writing to the thread-local `errno` location is sound.
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    use ::sysapi::{
        ffi::c_void,
        sys_ioctl::TIOCSPGRP,
    };

    let mut arg: pid_t = pgrp;
    let arg_ptr: *mut c_void = ::core::ptr::addr_of_mut!(arg).cast();
    // SAFETY: `TIOCSPGRP` reads a single `pid_t` through `arg_ptr`, which points to `arg`.
    match unsafe { crate::sys::ioctl::ioctl(fd, TIOCSPGRP, arg_ptr) } {
        Ok(_) => 0,
        Err(e) => {
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1
        },
    }
}
