// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
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
/// Returns the foreground process-group ID associated with the terminal referred to by `fd`. This
/// is implemented through the `TIOCGPGRP` ioctl, which the process manager daemon answers as the
/// owner of the controlling terminal's foreground group.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to the controlling terminal.
///
/// # Returns
///
/// The foreground process-group ID, or `-1` cast to `pid_t` with `errno` set on failure.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn tcgetpgrp(fd: c_int) -> pid_t {
    use ::sysapi::{
        ffi::c_void,
        sys_ioctl::TIOCGPGRP,
    };

    let mut pgrp: pid_t = 0;
    let arg_ptr: *mut c_void = ::core::ptr::addr_of_mut!(pgrp).cast();
    // SAFETY: `TIOCGPGRP` writes a single `pid_t` through `arg_ptr`, which points to `pgrp`.
    match unsafe { crate::sys::ioctl::ioctl(fd, TIOCGPGRP, arg_ptr) } {
        Ok(_) => pgrp,
        Err(e) => {
            // SAFETY: writing to the thread-local `errno` location is sound.
            unsafe {
                *__errno_location() = e.code.get();
            }
            -1 as pid_t
        },
    }
}
