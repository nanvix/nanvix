// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_statvfs::statvfs as statvfs_t,
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Retrieves file-system statistics for the file system that contains the file named by
/// `path` and stores them in the `statvfs` structure pointed to by `buf`.
///
/// # Parameters
///
/// - `path`: Null-terminated pathname of any file within the queried file system.
/// - `buf`: Pointer to a `struct statvfs` to be filled in on success.
///
/// # Returns
///
/// On success returns `0` and populates `*buf`. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// It exists so that consumers which only reference the symbol (notably libstdc++'s
/// `std::filesystem::space()`) link successfully; such callers treat the `-1`/`errno`
/// failure as "information unavailable". A future implementation should query the backing
/// file-system daemon and populate the block / inode counts and the mount flags.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers.
/// It is safe to call this function if `path` points to a valid, null-terminated C string
/// and `buf` (when non-null) points to writable storage large enough for a `struct statvfs`
/// in a future, fully implemented version.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn statvfs(_path: *const c_char, _buf: *mut statvfs_t) -> c_int {
    ::syslog::debug!("statvfs(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Retrieves file-system statistics for the file system that contains the open file
/// referred to by `fd` and stores them in the `statvfs` structure pointed to by `buf`.
///
/// # Parameters
///
/// - `fd`: An open file descriptor referring to any file within the queried file system.
/// - `buf`: Pointer to a `struct statvfs` to be filled in on success.
///
/// # Returns
///
/// On success returns `0` and populates `*buf`. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not
/// implemented). It mirrors [`statvfs()`] and exists so that portable software which
/// references the symbol compiles and links; such callers treat the `-1`/`errno` failure
/// as "information unavailable". A future implementation should query the backing
/// file-system daemon.
///
/// # Safety
///
/// This function is safe to call with any arguments; it ignores `fd` and `buf`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn fstatvfs(_fd: c_int, _buf: *mut statvfs_t) -> c_int {
    ::syslog::debug!("fstatvfs(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
