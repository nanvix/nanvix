// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    errno::__errno_location,
    ffi::c_char,
    sys_stat,
};
use ::syslog::trace_syscall;
use sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Obtains information about the file named `pathname`.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::sys::stat::stat`]
///
/// # Safety
///
/// This function has undefined because it dereferences a raw pointer (ie. `statbuf`).
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn stat(pathname: *const c_char, statbuf: *mut sys_stat::stat) -> c_int {
    // Convert C string to Rust string.
    let pathname: &str = match core::ffi::CStr::from_ptr(pathname).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!("stat(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let statbuf: &mut sys_stat::stat = &mut *statbuf;

    // Check if the path matches an in-memory filesystem mount.
    #[cfg(feature = "memfs")]
    {
        if crate::memfs::is_memfs_path(pathname) {
            if let Ok(info) = fat32::stat(pathname) {
                // Zero-initialize the stat buffer.
                ::core::ptr::write_bytes(statbuf as *mut sys_stat::stat, 0, 1);
                statbuf.st_size = info.size as ::sysapi::sys_types::off_t;
                statbuf.st_mode = if info.is_dir { 0o40755 } else { 0o100444 };
                statbuf.st_blksize = 4096;
                statbuf.st_blocks = ((info.size + 511) / 512) as ::sysapi::sys_types::off_t;
                return 0;
            }
            *__errno_location() = ErrorCode::NoSuchEntry.get();
            return -1;
        }
    }

    match crate::sys::stat::stat(pathname, statbuf) {
        Ok(_) => 0,
        Err(error) => {
            ::syslog::error!(
                "stat(): failed (pathname={}, statbuf={:p}, error={:?})",
                pathname,
                statbuf,
                error
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
