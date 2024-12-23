// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
/// TODO: Change function signature to use a variable argument list.
///
#[no_mangle]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: u32) -> c_int {
    ::nvx::log!("open(): path = {:?}, flags = {}, mode = {}", path, flags, mode);
    crate::unistd::open(path, flags, mode)
}
