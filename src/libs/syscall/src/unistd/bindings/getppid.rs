// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::pid_t;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the process ID of the parent of the calling process. The parent relationship is
/// established by `fork()` and tracked by the process daemon, so this function is only meaningful in
/// standalone deployment mode.
///
/// # Returns
///
/// Upon successful completion, `getppid()` returns the process ID of the parent of the calling
/// process. On failure, it returns `-1` cast to `pid_t`. In deployment modes other than standalone,
/// `getppid()` is not supported and sets `errno` to `ENOSYS`.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getppid() -> pid_t {
    #[cfg(not(feature = "standalone"))]
    {
        use crate::errno::__errno_location;
        use ::sys::error::ErrorCode;

        ::syslog::debug!("getppid(): not supported");
        // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }

    #[cfg(feature = "standalone")]
    {
        match ::proc::get_parent() {
            Ok(parent) => i32::from(parent),
            Err(e) => {
                // POSIX does not allow us to modify `errno`. So we just emit a warning.
                ::syslog::warn!("getppid(): failed (error={:?})", e);
                // POSIX does not reserve specific values for errors. We workaround it and return
                // `-1` to indicate an error.
                -1 as pid_t
            },
        }
    }
}
