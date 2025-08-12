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
pub unsafe extern "C" fn mmap(
    addr: *mut u8,
    length: usize,
    prot: i32,
    flags: i32,
    _fd: i32,
    offset: isize,
) -> *mut u8 {
    ::syslog::trace!(
        "mmap(): addr={addr:?}, length={length}, prot={prot}, flags={flags}, fd={_fd}, \
         offset={offset}"
    );

    ::syslog::error!("mmap(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    core::ptr::null_mut()
}
