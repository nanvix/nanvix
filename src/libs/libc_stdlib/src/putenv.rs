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
mod tests;
