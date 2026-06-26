// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::env_table;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Removes every variable from the environment, leaving it empty. After a successful call, the
/// C-visible `environ` array points at an empty (null-terminated) list and subsequent calls to
/// `getenv()` return a null pointer until the environment is repopulated.
///
/// # Returns
///
/// Returns `0` on success. A non-zero value is returned on failure; this implementation always
/// succeeds.
///
/// # Safety
///
/// This function is unsafe because it modifies global state. It is safe to call this function if
/// and only if access to the environment is synchronized with other threads that may read or
/// modify it.
///
/// # References
///
/// - https://man7.org/linux/man-pages/man3/clearenv.3.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn clearenv() -> c_int {
    env_table::clear();
    0
}
