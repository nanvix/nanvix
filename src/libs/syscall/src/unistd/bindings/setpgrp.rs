// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the process group of the calling process, equivalent to `setpgid(0, 0)`: the caller becomes
/// the leader of a new process group. In standalone mode this is routed to the process manager
/// daemon; in hosted modes it reports success without acting.
///
/// # Returns
///
/// `0` on success, or `-1` with `errno` set on failure.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn setpgrp() -> c_int {
    #[cfg(feature = "standalone")]
    {
        use crate::errno::__errno_location;
        use ::sys::pm::ProcessIdentifier;

        match ::proc::setpgid(ProcessIdentifier::from(0), ProcessIdentifier::from(0)) {
            Ok(()) => 0,
            Err(e) => {
                ::syslog::warn!("setpgrp(): failed (error={:?})", e);
                // SAFETY: writing to the thread-local `errno` location is sound.
                unsafe {
                    *__errno_location() = e.code.get();
                }
                -1
            },
        }
    }
    #[cfg(not(feature = "standalone"))]
    {
        0
    }
}
