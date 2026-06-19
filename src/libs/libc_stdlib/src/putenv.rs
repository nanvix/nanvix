// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    env_table,
    set_errno,
};
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

/// Adds or changes the environment variable described by `string`, which must have the form
/// `"NAME=VALUE"`. If `string` contains no `'='`, the named variable is removed.
///
/// # Description
///
/// The string pointed to by `string` becomes part of the environment, so subsequent modifications
/// to that storage are reflected in later environment lookups.
///
/// # Safety
///
/// `string` must be a valid, null-terminated string.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn putenv(string: *mut c_char) -> c_int {
    if string.is_null() {
        set_errno(EINVAL);
        return -1;
    }

    let mut p: *mut c_char = string;
    while *p != 0 {
        if *p as u8 == b'=' {
            return match env_table::put_raw(string) {
                Ok(()) => 0,
                Err(()) => {
                    set_errno(EINVAL);
                    -1
                },
            };
        }
        p = p.add(1);
    }

    crate::unsetenv::unsetenv(string)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::putenv;
    use crate::{
        getenv::getenv,
        set_errno,
    };
    use ::sysapi::{
        errno::EINVAL,
        ffi::{
            c_char,
            c_int,
        },
    };

    fn get_errno() -> c_int {
        unsafe { *::sysapi::errno::__errno_location() }
    }

    /// Returns `true` if the C string at `ptr` equals `expected` (excluding the terminator).
    unsafe fn cstr_eq(ptr: *const c_char, expected: &[u8]) -> bool {
        if ptr.is_null() {
            return false;
        }
        for (i, &b) in expected.iter().enumerate() {
            if *ptr.add(i) as u8 != b {
                return false;
            }
        }
        *ptr.add(expected.len()) as u8 == 0
    }

    #[test]
    fn sets_variable() {
        // Use a unique name to avoid colliding with other tests that share the
        // process-global environment table.
        let entry = b"NANVIX_PUTENV_SET=hello\0";
        assert_eq!(unsafe { putenv(entry.as_ptr().cast::<c_char>().cast_mut()) }, 0);

        let name = b"NANVIX_PUTENV_SET\0";
        let value = unsafe { getenv(name.as_ptr().cast::<c_char>()) };
        assert!(unsafe { cstr_eq(value, b"hello") });
    }

    #[test]
    fn uses_caller_storage() {
        let mut entry = *b"NANVIX_PUTENV_MUT=before\0";
        assert_eq!(unsafe { putenv(entry.as_mut_ptr().cast::<c_char>()) }, 0);

        let value_start: usize = b"NANVIX_PUTENV_MUT=".len();
        entry[value_start..value_start + b"second".len()].copy_from_slice(b"second");

        let name = b"NANVIX_PUTENV_MUT\0";
        let value = unsafe { getenv(name.as_ptr().cast::<c_char>()) };
        assert!(unsafe { cstr_eq(value, b"second") });

        // `putenv()` installs a pointer to the caller's storage in the process-global environment
        // table. Remove the entry while `entry` is still alive so the table never retains a
        // dangling pointer into this stack frame, which a later (or concurrent) environment lookup
        // would otherwise dereference.
        let remove = b"NANVIX_PUTENV_MUT\0";
        assert_eq!(unsafe { putenv(remove.as_ptr().cast::<c_char>().cast_mut()) }, 0);
    }

    #[test]
    fn removes_variable_without_equals() {
        let entry = b"NANVIX_PUTENV_DEL=value\0";
        assert_eq!(unsafe { putenv(entry.as_ptr().cast::<c_char>().cast_mut()) }, 0);

        // A string without '=' removes the named variable.
        let remove = b"NANVIX_PUTENV_DEL\0";
        assert_eq!(unsafe { putenv(remove.as_ptr().cast::<c_char>().cast_mut()) }, 0);

        let value = unsafe { getenv(remove.as_ptr().cast::<c_char>()) };
        assert!(value.is_null());
    }

    #[test]
    fn null_string_fails() {
        set_errno(0);
        assert_eq!(unsafe { putenv(::core::ptr::null_mut()) }, -1);
        assert_eq!(get_errno(), EINVAL);
    }
}
