// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_void,
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Performs a file tree walk starting at `dirpath` and calls the user-supplied callback
/// function `fn_cb` for each encountered file or directory. The walk is performed in
/// unspecified order and is limited by `nopenfd`, which specifies the maximum number of
/// file descriptors that may be used simultaneously during the traversal.
///
/// # Parameters
///
/// - `dirpath`: Null-terminated path to the starting directory.
/// - `fn_cb`: Callback invoked for each entry. Receives: the entry path, a pointer to a
///   `stat`-like structure (platform specific) and a type flag describing the entry kind.
/// - `nopenfd`: Maximum number of file descriptors to keep open while traversing.
///
/// # Returns
///
/// Returns `0` on success. On failure it returns `-1` and sets `errno` to indicate the
/// error. If the callback returns a non-zero value, a future compliant implementation
/// would stop the walk and propagate that value as the return code of `ftw()`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not
/// implemented). A future implementation should:
/// - Perform a depth-first (or specified order) traversal of the directory tree.
/// - Invoke the callback for each file, directory, symbolic link, etc.
/// - Enforce `nopenfd` by closing directories when descending beyond the limit.
/// - Populate and pass a proper `stat` structure to the callback.
/// - Map filesystem errors to appropriate `errno` values and continue or abort
///   traversal according to specification and callback return values.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign
/// callers and invokes a function pointer. It is safe to call this function if `dirpath`
/// is a valid, null-terminated string and `fn_cb` (when non-null) points to a callable
/// function with the expected signature in a future, fully implemented version.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn ftw(
    dirpath: *const c_char,
    fn_cb: Option<unsafe extern "C" fn(*const c_char, *const c_void, c_int) -> c_int>,
    nopenfd: c_int,
) -> c_int {
    ::syslog::debug!("ftw(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
