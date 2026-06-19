// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::env_table;
use ::core::ffi;
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Searches the environment list for a string of the form `name=value` and returns a pointer to
/// the value portion of the matched string. The returned pointer refers to internal storage that
/// remains valid until the next call to `setenv()` or `unsetenv()` that modifies the same
/// variable. The caller must not free or modify the memory pointed to by the return value.
///
/// # Parameters
///
/// - `name`: A pointer to a null-terminated string containing the name of the environment variable
///   to look up. Must not be null.
///
/// # Returns
///
/// A pointer to the value associated with the matched environment variable, or a null pointer if
/// the variable is not found or if `name` is null or invalid.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if and only if:
/// - `name` points to a valid null-terminated string.
/// - `name` remains valid for the duration of the function call.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getenv(name: *const c_char) -> *mut c_char {
    // Check if `name` is null.
    if name.is_null() {
        warn!("getenv(): name is null");
        return ::core::ptr::null_mut();
    }

    // Attempt to convert `name` to a Rust string.
    let name_str: &str = match ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => {
            warn!("getenv(): invalid name (name={name:?})");
            return ::core::ptr::null_mut();
        },
    };

    // Empty name or name containing '=' is invalid per POSIX.
    if name_str.is_empty() || name_str.contains('=') {
        warn!("getenv(): invalid name (name={name_str:?})");
        return ::core::ptr::null_mut();
    }

    // Look up the variable.
    let ptr: *const c_char = env_table::get(name_str);
    if ptr.is_null() {
        return ::core::ptr::null_mut();
    }

    ptr.cast_mut()
}
