// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the buffering of `stream`. Nanvix streams are unbuffered (each operation issues a direct
/// system call), so this is a no-op.
///
/// # Safety
///
/// `stream` must be null or a valid [`FILE`]; it is not dereferenced.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/setbuf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn setbuf(_stream: *mut FILE, _buf: *mut c_char) {}

///
/// # Description
///
/// Sets the buffering mode of `stream`. Nanvix streams are unbuffered, so this is a no-op that
/// reports success.
///
/// # Returns
///
/// Zero.
///
/// # Safety
///
/// `stream` must be null or a valid [`FILE`]; it is not dereferenced.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/setvbuf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn setvbuf(
    _stream: *mut FILE,
    _buf: *mut c_char,
    _mode: c_int,
    _size: c_size_t,
) -> c_int {
    0
}
