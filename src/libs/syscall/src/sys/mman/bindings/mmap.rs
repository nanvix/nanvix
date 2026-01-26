// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::mman::{
        self,
        MemoryMapFlags,
        MemoryMapProtectionFlags,
    },
};
use ::sys::{
    error::ErrorCode,
    mm::Address,
};
use ::sysapi::{
    ffi::c_int,
    sys_mman::MAP_FAILED,
    sys_types::{
        c_size_t,
        off_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps files or devices into memory. The `mmap()` function creates a new mapping in the virtual
/// address space of the calling process. The starting address for the new mapping is selected by
/// the system unless the `MAP_FIXED` flag is used.  The mapping can control page protections via
/// `prot` and mapping behavior via `flags`.
///
/// # Parameters
///
/// - `addr`: Address hint for the mapping. If `NULL`, the system chooses the address.
/// - `length`: Length in bytes of the mapping; must be greater than zero.
/// - `prot`: Desired memory protection of the mapping; bitwise OR of flags such as `PROT_NONE`,
///   `PROT_READ`, `PROT_WRITE`, and `PROT_EXEC`.
/// - `flags`: Determines the type of mapping; typically includes one of `MAP_PRIVATE` or
///   `MAP_SHARED` and may include `MAP_FIXED` and `MAP_ANONYMOUS`.
/// - `fd`: File descriptor to map.
/// - `offset`: Offset within the file. POSIX requires it to be a multiple of the page size for
///   file-backed mappings.
///
/// # Returns
///
/// On success, returns a pointer to the mapped area. On failure, returns `MAP_FAILED` and sets
/// `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global state.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - Access to `errno` is synchronized with other threads that may modify it.
///
/// # Known Limitations (Nanvix)
///
/// - Only anonymous, private mappings are supported. File-backed mappings are not implemented.
/// - `MAP_SHARED` is not supported.
/// - `MAP_FIXED` is not supported.
/// - The `addr` argument is always ignored.
/// - The `offset` parameter is always ignored.
/// - For anonymous mappings, `fd` must be `-1`.
/// - Any flags other than `MAP_PRIVATE | MAP_ANONYMOUS` are not supported.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn mmap(
    addr: *mut u8,
    length: c_size_t,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: off_t,
) -> *mut u8 {
    // Check if mapping length is invalid.
    if length == 0 {
        ::syslog::error!(
            "mmap(): invalid mapping length (addr={addr:?}, length={length}), prot={prot}, \
             flags={flags}, fd={fd}, offset={offset})"
        );
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return MAP_FAILED;
    }

    // Attempt to convert length.
    let length: usize = match TryFrom::try_from(length) {
        Ok(length) => length,
        Err(error) => {
            ::syslog::error!(
                "mmap(): invalid mapping length (addr={addr:?}, length={length}), prot={prot}, \
                 flags={flags}, fd={fd}, offset={offset}), error={error:?}"
            );
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return MAP_FAILED;
        },
    };

    // Check if address hint was provided.
    if !addr.is_null() {
        ::syslog::warn!(
            "mmap(): address hint is not supported and will be ignored (addr={addr:?}, \
             length={length}, prot={prot}, flags={flags}, fd={fd}, offset={offset})"
        );
    }

    // Attempt to convert flags.
    let flags: MemoryMapFlags = match TryFrom::try_from(flags) {
        Ok(flags) => flags,
        Err(error) => {
            ::syslog::error!(
                "mmap(): invalid flags (addr={addr:?}, length={length}, prot={prot}, \
                 flags={flags}, fd={fd}, offset={offset}), error={error:?}"
            );
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return MAP_FAILED;
        },
    };

    // Check for unsupported flags.
    if flags.is_shared() {
        ::syslog::error!(
            "mmap(): shared memory mapping is not supported (addr={addr:?}, length={length}, \
             prot={prot}, flags={flags:?}, fd={fd}, offset={offset})"
        );
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return MAP_FAILED;
    } else if flags.is_fixed() {
        ::syslog::error!(
            "mmap(): fixed memory mapping is not supported (addr={addr:?}, length={length}, \
             prot={prot}, flags={flags:?}, fd={fd}, offset={offset})"
        );
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return MAP_FAILED;
    } else if !flags.is_anonymous() {
        ::syslog::error!(
            "mmap(): non-anonymous memory mapping is not supported (addr={addr:?}, \
             length={length}, prot={prot}, flags={flags:?}, fd={fd}, offset={offset})"
        );
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return MAP_FAILED;
    } else if flags.is_anonymous() && fd != -1 {
        ::syslog::error!(
            "mmap(): anonymous memory mapping with file descriptor is not supported \
             (addr={addr:?}, length={length}, prot={prot}, flags={flags:?}, fd={fd}, \
             offset={offset})"
        );
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return MAP_FAILED;
    }

    // Check for undefined behavior.
    if flags.is_anonymous() && fd == -1 && offset != 0 {
        ::syslog::warn!(
            "mmap(): offset is ignored for anonymous memory mapping (addr={addr:?}, \
             length={length}, prot={prot}, flags={flags:?}, fd={fd}, offset={offset})"
        );
    }

    // Attempt to convert protection flags.
    let prot: MemoryMapProtectionFlags = match TryFrom::try_from(prot) {
        Ok(prot) => prot,
        Err(error) => {
            ::syslog::error!(
                "mmap(): invalid protection flags (addr={addr:?}, length={length}, prot={prot}, \
                 flags={flags:?}, fd={fd}, offset={offset}), error={error:?}"
            );
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return MAP_FAILED;
        },
    };

    // Map memory region and check for errors.
    match mman::mmap(length, prot) {
        Ok(virt_addr) => {
            ::syslog::trace!(
                "mmap(): success (addr={addr:?}, length={length}, prot={prot:?}, flags={flags:?}, \
                 fd={fd}, offset={offset}), virt_addr={virt_addr:?}"
            );
            virt_addr.into_raw_value() as *mut u8
        },
        Err(error) => {
            ::syslog::error!(
                "mmap(): failed (addr={addr:?}, length={length}, prot={prot:?}, flags={flags:?}, \
                 fd={fd}, offset={offset}), error={error:?}"
            );
            unsafe {
                *__errno_location() = error.code.get();
            }
            MAP_FAILED
        },
    }
}
