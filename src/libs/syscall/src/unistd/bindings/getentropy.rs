// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_int,
    c_void,
};
use ::sysapi::sys_types::size_t;
use core::slice;

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getentropy(buffer: *mut c_void, length: size_t) -> c_int {
    ::syslog::trace!("getentropy(): buffer = {:?}, length = {}", buffer, length);

    // Fill buffer with 1s.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, length);
    for byte in buffer.iter_mut() {
        *byte = 1;
    }

    0
}