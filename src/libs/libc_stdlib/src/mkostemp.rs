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
    fcntl::{
        file_access_mode::O_RDWR,
        file_creation_flags::{
            O_CREAT,
            O_EXCL,
        },
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_stat::file_mode::{
        S_IRUSR,
        S_IWUSR,
    },
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
/// Generates a unique temporary file name from `template`, creates and opens the file with mode
/// `0600`, and returns its file descriptor. The trailing `XXXXXX` of `template` is replaced in place
/// with the characters used in the successful name. The file is created with `O_EXCL`, so it is
/// guaranteed not to have existed beforehand. Additional `flag` bits are passed to `open()` together
/// with `O_RDWR | O_CREAT | O_EXCL`.
///
/// # Parameters
///
/// - `template`: Pointer to a modifiable null-terminated string ending in `XXXXXX`.
/// - `flag`: Additional flags to pass to `open()`.
///
/// # Returns
///
/// An open file descriptor on success, or `-1` on error with `errno` set (`EINVAL` if the template
/// does not end in `XXXXXX`, or the error reported by `open()`).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `template` points to a writable null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkostemp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mkostemp(template: *mut c_char, flag: c_int) -> c_int {
    unsafe extern "C" {
        fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int;
    }

    let Some(len) = tmpname::validate(template) else {
        set_errno(EINVAL);
        return -1;
    };

    let mut attempts: u32 = 0;
    while attempts < MAX_ATTEMPTS {
        // SAFETY: validate() confirmed the template is at least SUFFIX_LEN bytes long.
        unsafe { tmpname::randomize_suffix(template, len) };

        // SAFETY: template is a valid null-terminated path; flags and mode are valid.
        let fd: c_int =
            unsafe { open(template, flag | O_RDWR | O_CREAT | O_EXCL, S_IRUSR | S_IWUSR) };
        if fd >= 0 {
            return fd;
        }

        // Only a name collision warrants another attempt; any other error is fatal and `open()`
        // has already set `errno`.
        // SAFETY: __errno_location() returns a valid pointer to errno.
        if unsafe { *__errno_location() } != EEXIST {
            return -1;
        }

        attempts += 1;
    }

    set_errno(EEXIST);
    -1
}
