// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    errno::{
        EBADF,
        EFAULT,
        EINVAL,
    },
    ffi::{
        c_char,
        c_int,
        c_long,
    },
    limits::{
        NAME_MAX,
        PATH_MAX,
    },
    unistd::pathconf_names::{
        PC_2_SYMLINKS,
        PC_ALLOC_SIZE_MIN,
        PC_ASYNC_IO,
        PC_CHOWN_RESTRICTED,
        PC_FILESIZEBITS,
        PC_LINK_MAX,
        PC_MAX_CANON,
        PC_MAX_INPUT,
        PC_NAME_MAX,
        PC_NO_TRUNC,
        PC_PATH_MAX,
        PC_PIPE_BUF,
        PC_PRIO_IO,
        PC_REC_INCR_XFER_SIZE,
        PC_REC_MAX_XFER_SIZE,
        PC_REC_MIN_XFER_SIZE,
        PC_REC_XFER_ALIGN,
        PC_SYMLINK_MAX,
        PC_SYNC_IO,
        PC_VDISABLE,
    },
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

//==================================================================================================
// Private Functions
//==================================================================================================

/// Shared limit lookup for `pathconf()`/`fpathconf()`. The values are system-global on Nanvix today
/// (there are no per-file-system overrides yet), so the path/descriptor argument is unused.
///
/// Returns the configured limit for a known selector, `-1` **without modifying `errno`** for an
/// option that has no determinate limit (so callers take the "no limit -> fall back" branch), and
/// `-1` with `errno = EINVAL` for an unrecognized selector (per POSIX, not `ENOSYS`).
///
/// # Safety
///
/// Writes to the thread-local `errno` via `__errno_location()` for the unknown-selector case.
unsafe fn pathconf_value(name: c_int) -> c_long {
    match name {
        // Selectors with determinate limits (mirroring <limits.h>).
        PC_PATH_MAX | PC_SYMLINK_MAX => PATH_MAX as c_long, // 1024
        PC_NAME_MAX => NAME_MAX as c_long,                  // 255
        PC_LINK_MAX => 32767,                               // implementation-defined upper bound
        PC_MAX_CANON | PC_MAX_INPUT => 255,                 // _POSIX_MAX_CANON / _POSIX_MAX_INPUT
        PC_PIPE_BUF => 4096,
        PC_FILESIZEBITS => 64,
        PC_CHOWN_RESTRICTED | PC_NO_TRUNC | PC_2_SYMLINKS => 1,
        PC_VDISABLE => 0,
        // Options with no determinate limit: return -1 WITHOUT touching errno so callers (e.g.
        // libc++ <filesystem>) take the "no limit -> fall back" branch.
        PC_SYNC_IO
        | PC_ASYNC_IO
        | PC_PRIO_IO
        | PC_ALLOC_SIZE_MIN
        | PC_REC_INCR_XFER_SIZE
        | PC_REC_MAX_XFER_SIZE
        | PC_REC_MIN_XFER_SIZE
        | PC_REC_XFER_ALIGN => -1,
        // Unrecognized selector: POSIX mandates EINVAL (not ENOSYS).
        _ => {
            *__errno_location() = EINVAL;
            -1
        },
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Retrieves the value of a configurable system limit or option associated with the
/// pathname `path`, as identified by `name` (one of the `_PC_*` selectors defined in
/// `<unistd.h>`).
///
/// # Parameters
///
/// - `path`: Null-terminated pathname of the file or directory being queried.
/// - `name`: A `_PC_*` selector specifying which configurable value to retrieve.
///
/// # Returns
///
/// On success the configured limit for `name`; `-1` (with `errno` unchanged) when the queried
/// option has no determinate limit; `-1` with `errno = EFAULT` when `path` is NULL; or `-1` with
/// `errno = EINVAL` for an unrecognized selector.
///
/// # Notes
///
/// The limits are system-global on Nanvix today, so `path` is not consulted beyond a NULL check; it
/// is only required to be a valid pointer. A future implementation could consult the VFS for
/// per-file-system limits.
///
/// # Safety
///
/// This function is unsafe because it accepts a raw pointer supplied by foreign callers.
/// It is safe to call this function if `path` (when non-null) points to a valid
/// null-terminated C string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pathconf(path: *const c_char, name: c_int) -> c_long {
    if path.is_null() {
        *__errno_location() = EFAULT;
        return -1;
    }
    pathconf_value(name)
}

///
/// # Description
///
/// Retrieves the value of a configurable system limit or option associated with the
/// open file descriptor `fd`, as identified by `name` (one of the `_PC_*` selectors
/// defined in `<unistd.h>`).
///
/// # Parameters
///
/// - `fd`: An open file descriptor to query.
/// - `name`: A `_PC_*` selector specifying which configurable value to retrieve.
///
/// # Returns
///
/// On success the configured limit for `name`; `-1` (with `errno` unchanged) when the queried
/// option has no determinate limit; `-1` with `errno = EBADF` when `fd` is invalid; or `-1` with
/// `errno = EINVAL` for an unrecognized selector.
///
/// # Notes
///
/// The limits are system-global on Nanvix today, so `fd` is not consulted beyond a validity check.
/// Behaves identically to `pathconf()` for the same selector.
///
/// # Safety
///
/// This function is safe to call with any integer; a negative descriptor is rejected with `EBADF`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn fpathconf(fd: c_int, name: c_int) -> c_long {
    if fd < 0 {
        *__errno_location() = EBADF;
        return -1;
    }
    pathconf_value(name)
}
