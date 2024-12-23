// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::types::pid_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub extern "C" fn kill(_pid: pid_t, _signal: c_int) -> c_int {
    ::nvx::log!("kill(): pid = {}, signal = {}", _pid, _signal);
    // TODO: Implement this system call.
    -1
}
