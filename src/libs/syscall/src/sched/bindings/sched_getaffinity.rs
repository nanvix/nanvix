// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        pid_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Retrieves the CPU affinity mask of the thread identified by `pid` into the buffer pointed to by
/// `mask`. Nanvix schedules every process on a single logical processor, so the reported affinity
/// for the calling process always consists of exactly one online CPU (CPU 0). The remaining bits of
/// the mask are cleared.
///
/// # Parameters
///
/// - `pid`: The thread whose affinity is queried. A value of zero selects the calling process.
/// - `cpusetsize`: The size, in bytes, of the buffer pointed to by `mask`.
/// - `mask`: Buffer that receives the affinity mask.
///
/// # Returns
///
/// Upon successful completion, `sched_getaffinity()` returns `0`. On failure, `-1` is returned and
/// `errno` is set to indicate the error.
///
/// # Safety
///
/// The caller must ensure that `mask` points to a writable region of at least `cpusetsize` bytes.
///
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sched_getaffinity(
    pid: pid_t,
    cpusetsize: c_size_t,
    mask: *mut c_void,
) -> c_int {
    // A NULL destination buffer cannot receive the affinity mask.
    if mask.is_null() {
        *__errno_location() = ErrorCode::BadAddress.get();
        return -1;
    }

    // A zero-sized buffer is not a valid argument.
    if cpusetsize == 0 {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    if pid < 0 {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    if pid > 0 {
        match unistd::getpid() {
            Ok(self_pid) if pid == self_pid.into() => {},
            Ok(_) => {
                *__errno_location() = ErrorCode::NoSuchProcess.get();
                return -1;
            },
            Err(error) => {
                ::syslog::warn!("sched_getaffinity(pid={:?}): failed (error={:?})", pid, error);
                *__errno_location() = error.code.get();
                return -1;
            },
        }
    }

    let len: usize = cpusetsize as usize;
    // SAFETY: `mask` is non-null and the caller guarantees it points to at least `cpusetsize`
    // writable bytes. We clear the whole mask and then mark CPU 0 as the only online processor.
    unsafe {
        ::core::ptr::write_bytes(mask as *mut u8, 0, len);
        (mask as *mut u8).write(1);
    }

    0
}
