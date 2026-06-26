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
        EEXIST,
        EINVAL,
    },
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of distinct names to try before giving up.
const MAX_ATTEMPTS: u32 = 128;

/// `access()` mode that tests only for the existence of a path.
const F_OK: c_int = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Generates a unique temporary file name from `template`. The trailing `XXXXXX` of `template` is
/// replaced in place with characters that make the resulting path not name an existing file. Unlike
/// `mkstemp()`, the file is **not** created, so the name may be claimed by another process before
/// the caller uses it; `mktemp()` is therefore inherently racy and `mkstemp()` should be preferred.
///
/// # Parameters
///
/// - `template`: Pointer to a modifiable null-terminated string ending in `XXXXXX`.
///
/// # Returns
///
/// The `template` pointer is always returned. On success its trailing `XXXXXX` has been replaced
/// with a name that does not currently exist. On failure `template` is made an empty string and
/// `errno` is set (`EINVAL` if the template does not end in `XXXXXX`, or `EEXIST` if no unused name
/// could be found).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `template` points to a writable null-terminated string.
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man3/mktemp.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mktemp(template: *mut c_char) -> *mut c_char {
    unsafe extern "C" {
        fn access(path: *const c_char, amode: c_int) -> c_int;
    }

    let Some(len) = tmpname::validate(template) else {
        set_errno(EINVAL);
        // Per the mktemp() contract, the template is made an empty string on failure.
        if !template.is_null() {
            // SAFETY: template is non-null here.
            unsafe { *template = 0 };
        }
        return template;
    };

    let mut attempts: u32 = 0;
    while attempts < MAX_ATTEMPTS {
        // SAFETY: validate() confirmed the template is at least SUFFIX_LEN bytes long.
        unsafe { tmpname::randomize_suffix(template, len) };

        // A name that does not exist is suitable; `access(..., F_OK)` fails with ENOENT when the
        // candidate does not exist.
        // SAFETY: template is a valid null-terminated path.
        if unsafe { access(template, F_OK) } != 0 {
            // SAFETY: __errno_location() returns a valid pointer to errno.
            if unsafe { *::sysapi::errno::__errno_location() } == ::sysapi::errno::ENOENT {
                return template;
            }

            // Per the mktemp() contract, clear the template on failure.
            unsafe { *template = 0 };
            return template;
        }

        attempts += 1;
    }

    set_errno(EEXIST);
    // SAFETY: validate() succeeded, so template is non-null and writable.
    unsafe { *template = 0 };
    template
}
