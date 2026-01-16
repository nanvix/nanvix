// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::mman::{
        self,
        MemoryMapProtectionFlags,
    },
};
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
};
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn mprotect(addr: *mut c_char, length: c_size_t, prot: c_int) -> isize {
    // Check if address is invalid.
    if addr.is_null() {
        ::syslog::error!(
            "mprotect(): invalid base address (addr={addr:?}, length={length}, prot={prot})"
        );
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Check for zero length.
    if length == 0 {
        // No action to perform.
        return 0;
    }

    // Attempt to convert length.
    let length: usize = match usize::try_from(length) {
        Ok(length) => length,
        Err(_) => {
            ::syslog::error!(
                "mprotect(): invalid length (addr={addr:?}, length={length}, prot={prot})"
            );
            unsafe {
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
            }
            return -1;
        },
    };

    // Convert base pointer to virtual address.
    let base: VirtualAddress = VirtualAddress::from_raw_value(addr as usize);

    // Attempt to convert protection flags.
    let prot: MemoryMapProtectionFlags = match MemoryMapProtectionFlags::try_from(prot) {
        Ok(prot) => prot,
        Err(_) => {
            ::syslog::error!(
                "mprotect(): invalid protection flags (addr={addr:?}, length={length}, \
                 prot={prot})"
            );
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        },
    };

    // Attempt to set protection of memory mapping and check for errors.
    match mman::mprotect(base, length, prot) {
        Ok(()) => {
            ::syslog::trace!("mprotect(): success (addr={addr:?}, length={length}, prot={prot:?})");
            0
        },
        Err(error) => {
            ::syslog::error!(
                "mprotect(): failed (addr={addr:?}, length={length}, prot={prot:?}), \
                 error={error:?}"
            );
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}
