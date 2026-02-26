// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    errno::__errno_location,
    sys_stat,
};
use ::syslog::trace_syscall;
use sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Safety
///
/// This function has undefined behavior if buf points to an invalid memory location.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn fstat(fd: c_int, buf: *mut sys_stat::stat) -> c_int {
    // Check if this is an in-memory filesystem file descriptor.
    #[cfg(feature = "memfs")]
    {
        if crate::memfs::is_memfs_fd(fd) {
            match crate::memfs::memfs_fstat(fd, &mut *buf) {
                Ok(()) => return 0,
                Err(_) => {
                    *__errno_location() = ::sysapi::errno::EBADF;
                    return -1;
                },
            }
        }
    }

    match crate::sys::stat::fstat(fd, &mut *buf) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!("fstat(): failed (fd={}, buf={:p}, error={:?})", fd, buf, error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
