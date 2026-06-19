// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    env_table,
    set_errno,
};
use ::core::ffi;
use ::sysapi::{
    errno::EINVAL,
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Removes an environment variable from the environment list. If `name` does not exist in the
/// environment, the function succeeds without modifying the environment. After a successful call,
/// subsequent calls to `getenv()` with the same `name` will return a null pointer.
///
/// # Parameters
///
/// - `name`: A pointer to a null-terminated string containing the name of the environment variable
///   to remove. Must not be null, must not be empty, and must not contain `=`.
///
/// # Returns
///
/// Returns `0` on success. On error, returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and modifies global state.
///
/// It is safe to call this function if and only if:
/// - `name` points to a valid null-terminated string.
/// - `name` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn unsetenv(name: *const c_char) -> c_int {
    // Check if `name` is null.
    if name.is_null() {
        warn!("unsetenv(): name is null");
        set_errno(EINVAL);
        return -1;
    }

    // Attempt to convert `name`.
    let name_str: &str = match ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => {
            warn!("unsetenv(): invalid name (name={name:?})");
            set_errno(EINVAL);
            return -1;
        },
    };

    // Empty name or name containing '=' is invalid per POSIX.
    if name_str.is_empty() || name_str.contains('=') {
        warn!("unsetenv(): invalid name (name={name_str:?})");
        set_errno(EINVAL);
        return -1;
    }

    env_table::unset(name_str);
    0
}
