// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_errno,
    tmpname,
};
use ::sysapi::{
    errno::{
        __errno_location,
        EEXIST,
        EINVAL,
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_stat::file_mode::S_IRWXU,
    sys_types::mode_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of distinct names to try before giving up.
const MAX_ATTEMPTS: u32 = 128;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Generates a unique temporary directory name from `template` and creates the directory with mode
/// `0700`. The trailing `XXXXXX` of `template` is replaced in place with the characters used in the
/// successful name.
///
/// # Parameters
///
/// - `template`: Pointer to a modifiable null-terminated string ending in `XXXXXX`.
///
/// # Returns
///
/// `template` on success, or a null pointer on error with `errno` set (`EINVAL` if the template
/// does not end in `XXXXXX`, or the error reported by `mkdir()`).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `template` points to a writable null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkdtemp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mkdtemp(template: *mut c_char) -> *mut c_char {
    unsafe extern "C" {
        fn mkdir(path: *const c_char, mode: mode_t) -> c_int;
    }

    let Some(len) = tmpname::validate(template) else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };

    let mut attempts: u32 = 0;
    while attempts < MAX_ATTEMPTS {
        // SAFETY: validate() confirmed the template is at least SUFFIX_LEN bytes long.
        unsafe { tmpname::randomize_suffix(template, len) };

        // SAFETY: template is a valid null-terminated path; mode is valid.
        if unsafe { mkdir(template, S_IRWXU) } == 0 {
            return template;
        }

        // Only a name collision warrants another attempt; any other error is fatal and `mkdir()`
        // has already set `errno`.
        // SAFETY: __errno_location() returns a valid pointer to errno.
        if unsafe { *__errno_location() } != EEXIST {
            return core::ptr::null_mut();
        }

        attempts += 1;
    }

    set_errno(EEXIST);
    core::ptr::null_mut()
}
