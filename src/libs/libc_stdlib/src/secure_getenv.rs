// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets an environment variable when the process is not running in a privileged context.
///
/// Nanvix does not currently support set-user-ID or set-group-ID execution, so the security
/// criteria are equivalent to those of [`crate::getenv`].
///
/// # Parameters
///
/// - `name`: Environment-variable name.
///
/// # Returns
///
/// A pointer to the value associated with `name`, or a null pointer if not found.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers through [`crate::getenv`].
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/getenv.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn secure_getenv(name: *const c_char) -> *mut c_char {
    crate::getenv(name)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::secure_getenv;
    use ::sysapi::ffi::c_char;

    #[test]
    fn missing_variable_returns_null() {
        let name = b"NANVIX_SECURE_GETENV_MISSING\0";
        let ptr: *mut c_char = unsafe { secure_getenv(name.as_ptr().cast::<c_char>()) };
        assert!(ptr.is_null());
    }
}
