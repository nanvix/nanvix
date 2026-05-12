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
/// Adds or changes an environment variable. If the variable `name` does not exist in the
/// environment, a new `name=value` entry is created. If `name` already exists and `overwrite` is
/// non-zero, the existing value is replaced with `value`. If `name` already exists and `overwrite`
/// is zero, the existing value is not changed and the function succeeds.
///
/// # Parameters
///
/// - `name`: A pointer to a null-terminated string containing the variable name. Must not be null,
///   must not be empty, and must not contain `=`.
/// - `value`: A pointer to a null-terminated string containing the variable value. Must not be
///   null.
/// - `overwrite`: If non-zero, replace an existing variable; if zero, keep the existing value.
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
/// - `value` points to a valid null-terminated string.
/// - Both `name` and `value` remain valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn setenv(
    name: *const c_char,
    value: *const c_char,
    overwrite: c_int,
) -> c_int {
    // Check if `name` is null.
    if name.is_null() {
        ::syslog::warn!("setenv(): name is null");
        set_errno(EINVAL);
        return -1;
    }

    // Check if `value` is null.
    if value.is_null() {
        ::syslog::warn!("setenv(): value is null");
        set_errno(EINVAL);
        return -1;
    }

    // Attempt to convert `name`.
    let name_str: &str = match ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::warn!("setenv(): invalid name (name={name:?})");
            set_errno(EINVAL);
            return -1;
        },
    };

    // Attempt to convert `value`.
    let value_str: &str = match ffi::CStr::from_ptr(value).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::warn!("setenv(): invalid value (value={value:?})");
            set_errno(EINVAL);
            return -1;
        },
    };

    // Attempt to set the variable. The registered callback (if any) is invoked by env_table::set()
    // when the value is actually written.
    let should_overwrite: bool = overwrite != 0;
    match env_table::set(name_str, value_str, should_overwrite) {
        Ok(_) => 0,
        Err(()) => {
            ::syslog::warn!("setenv(): failed (name={name_str:?}, value={value_str:?})");
            set_errno(EINVAL);
            -1
        },
    }
}
