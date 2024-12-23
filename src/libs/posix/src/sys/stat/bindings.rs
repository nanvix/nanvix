// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::stat,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Safety
///
/// This function has undefined behavior if buf points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn fstat(fd: c_int, buf: *mut stat::stat) -> c_int {
    ::nvx::log!("fstat(): fd = {}, buf = {:?}", fd, buf);
    crate::sys::stat::fstat(fd, &mut *buf)
}
