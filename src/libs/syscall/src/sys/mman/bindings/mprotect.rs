// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::errno::__errno_location;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mprotect(addr: *mut u8, length: usize, prot: i32) -> isize {
    ::syslog::trace!("mprotect(): addr={addr:?}, length={length}, prot={prot}");

    ::syslog::error!("mprotect(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
