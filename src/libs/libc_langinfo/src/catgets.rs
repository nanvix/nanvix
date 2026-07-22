// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
pub use ::sysapi::nl_types::nl_catd;
use ::sysapi::{
    errno::ENOENT,
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
/// Opens the message catalog identified by `name`.
///
/// # Parameters
///
/// - `name`: Name of the message catalog to open.
/// - `oflag`: Flag selecting the locale used to interpret the catalog.
///
/// # Returns
///
/// Nanvix supports only the C/POSIX locale, which defines no message catalogs, so this function
/// always reports failure by returning `(nl_catd)-1` with `errno` set to `ENOENT`.
///
/// # Safety
///
/// This function is safe for all input values. The `name` argument is never dereferenced.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn catopen(name: *const c_char, oflag: c_int) -> nl_catd {
    let _ = (name, oflag);
    set_errno(ENOENT);
    -1isize as nl_catd
}

///
/// # Description
///
/// Reads a message from the message catalog `catd`.
///
/// # Parameters
///
/// - `catd`: Message catalog descriptor returned by [`catopen()`].
/// - `set_id`: Identifier of the message set to search.
/// - `msg_id`: Identifier of the message to read.
/// - `s`: Fallback string returned when the message cannot be found.
///
/// # Returns
///
/// Nanvix defines no message catalogs, so no message is ever found and this function returns the
/// fallback string `s` unchanged.
///
/// # Safety
///
/// This function is safe for all input values. The `s` argument is returned unchanged and is never
/// dereferenced.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn catgets(
    catd: nl_catd,
    set_id: c_int,
    msg_id: c_int,
    s: *const c_char,
) -> *mut c_char {
    let _ = (catd, set_id, msg_id);
    s.cast_mut()
}

///
/// # Description
///
/// Closes the message catalog `catd`.
///
/// # Parameters
///
/// - `catd`: Message catalog descriptor returned by [`catopen()`].
///
/// # Returns
///
/// Always returns zero, as no message catalog resources are ever allocated.
///
/// # Safety
///
/// This function is safe for all input values. The `catd` argument is never dereferenced.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn catclose(catd: nl_catd) -> c_int {
    let _ = catd;
    0
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        catclose,
        catgets,
        catopen,
        nl_catd,
    };

    #[test]
    fn test_catopen_reports_failure() {
        let catd: nl_catd = catopen(c"messages".as_ptr(), 0);
        assert_eq!(catd, -1isize as nl_catd);
    }

    #[test]
    fn test_catgets_returns_fallback() {
        let fallback = c"fallback".as_ptr();
        let catd: nl_catd = catopen(c"messages".as_ptr(), 0);
        assert_eq!(catgets(catd, 1, 1, fallback).cast_const(), fallback);
    }

    #[test]
    fn test_catclose_succeeds() {
        let catd: nl_catd = catopen(c"messages".as_ptr(), 0);
        assert_eq!(catclose(catd), 0);
    }
}
