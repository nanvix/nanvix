// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_long,
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

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
/// On success a non-negative limit value, or `-1` (with `errno` unchanged) when the
/// queried option has no determinate limit. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always returns `-1` with `errno = ENOSYS`,
/// matching the convention used by the other "not implemented" stubs in this module.
/// Callers (notably libstdc++'s `std::filesystem`) treat `-1` as "no limit known" and
/// fall back to compile-time defaults such as `PATH_MAX`, so this stub is sufficient
/// to satisfy the libstdc++ link without changing behaviour. A future implementation
/// should return real limits for the selectors it knows about (e.g. `_PC_PATH_MAX`,
/// `_PC_NAME_MAX`, `_PC_LINK_MAX`), and only set `errno = EINVAL` for genuinely
/// unrecognised selectors per the POSIX contract.
///
/// # Safety
///
/// This function is unsafe because it accepts a raw pointer supplied by foreign callers.
/// It is safe to call this function if `path` (when non-null) points to a valid
/// null-terminated C string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pathconf(_path: *const c_char, _name: c_int) -> c_long {
    ::syslog::debug!("pathconf(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
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
/// On success a non-negative limit value, or `-1` (with `errno` unchanged) when the
/// queried option has no determinate limit. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always returns `-1` with `errno = ENOSYS`.
/// See `pathconf()` for the rationale on why this stub is acceptable for the current
/// libstdc++ link requirements.
///
/// # Safety
///
/// This function is safe to call with any integer; passing a descriptor that is not
/// currently open does not change the (stub) behaviour.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn fpathconf(_fd: c_int, _name: c_int) -> c_long {
    ::syslog::debug!("fpathconf(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
