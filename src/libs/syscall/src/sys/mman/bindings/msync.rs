// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "syscall")]
use crate::{
    errno::__errno_location,
    sys::mman,
};
#[cfg(any(feature = "syscall", test))]
use ::sys::error::ErrorCode;
#[cfg(feature = "syscall")]
use ::sys::mm::VirtualAddress;
#[cfg(any(feature = "syscall", test))]
use ::sysapi::ffi::c_int;
#[cfg(feature = "syscall")]
use ::sysapi::ffi::c_void;
#[cfg(any(feature = "syscall", test))]
use ::sysapi::sys_mman::msync_flags::{
    MS_ASYNC,
    MS_INVALIDATE,
    MS_SYNC,
};
#[cfg(feature = "syscall")]
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Validates the flags supplied to the `msync()` binding.
///
/// POSIX requires `flags` to specify exactly one synchronization mode, `MS_ASYNC` or `MS_SYNC`,
/// optionally combined with `MS_INVALIDATE`. Any other combination, including no synchronization
/// mode at all or unknown bits, is rejected.
///
/// # Parameters
///
/// - `flags`: Synchronization flags supplied by the caller.
///
/// # Returns
///
/// Returns `Ok(())` if `flags` is a valid combination, otherwise the `ErrorCode` to report via
/// `errno`.
///
#[cfg(any(feature = "syscall", test))]
fn validate_msync_flags(flags: c_int) -> Result<(), ErrorCode> {
    // Reject any bit outside the set of recognized flags.
    const KNOWN: c_int = MS_ASYNC | MS_INVALIDATE | MS_SYNC;
    if flags & !KNOWN != 0 {
        return Err(ErrorCode::InvalidArgument);
    }

    // Exactly one synchronization mode must be requested.
    let mode: c_int = flags & (MS_ASYNC | MS_SYNC);
    if mode != MS_ASYNC && mode != MS_SYNC {
        return Err(ErrorCode::InvalidArgument);
    }

    Ok(())
}

///
/// # Description
///
/// Synchronizes the whole pages containing any part of the address range that starts at `addr` and
/// spans `length` bytes with their backing store.
///
/// Nanvix backs every mapping with anonymous physical memory, so the in-memory contents are always
/// the authoritative copy and there is nothing to write back. This function therefore validates its
/// arguments and reports success without performing any I/O.
///
/// # Parameters
///
/// - `addr`: Start address of the region to synchronize.
/// - `length`: Number of bytes to synchronize.
/// - `flags`: Synchronization flags (`MS_ASYNC`, `MS_SYNC`, and optionally `MS_INVALIDATE`).
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error:
/// - `EINVAL`: `flags` is invalid, or `addr` is not a multiple of the page size.
/// - `ENOMEM`: the range extends past the end of the address space, or some page in the range is
///   not mapped.
///
/// # Safety
///
/// This function is unsafe because it may modify `errno`. It is safe to call provided that access
/// to `errno` is synchronized with other threads that may modify it.
///
/// # Known Limitations (Nanvix)
///
/// - Synchronization is a no-op: mappings are anonymous and have no separate backing store.
///
#[cfg(feature = "syscall")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msync(addr: *mut c_void, length: c_size_t, flags: c_int) -> c_int {
    if let Err(code) = validate_msync_flags(flags) {
        unsafe {
            *__errno_location() = code.get();
        }
        return -1;
    }

    match super::validate_lock_range(addr as usize, length) {
        Ok(None) => 0,
        Ok(Some((addr, length))) => {
            let addr: VirtualAddress = VirtualAddress::from_raw_value(addr);
            match mman::msync(addr, length) {
                Ok(()) => 0,
                Err(error) => {
                    unsafe {
                        *__errno_location() = error.code.get();
                    }
                    -1
                },
            }
        },
        Err(code) => {
            unsafe {
                *__errno_location() = code.get();
            }
            -1
        },
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::validate_msync_flags;
    use ::sys::error::ErrorCode;
    use ::sysapi::sys_mman::msync_flags::{
        MS_ASYNC,
        MS_INVALIDATE,
        MS_SYNC,
    };

    /// A lone `MS_SYNC` flag is accepted.
    #[test]
    fn accepts_ms_sync() {
        assert_eq!(validate_msync_flags(MS_SYNC), Ok(()));
    }

    /// A lone `MS_ASYNC` flag is accepted.
    #[test]
    fn accepts_ms_async() {
        assert_eq!(validate_msync_flags(MS_ASYNC), Ok(()));
    }

    /// A synchronization mode combined with `MS_INVALIDATE` is accepted.
    #[test]
    fn accepts_mode_with_invalidate() {
        assert_eq!(validate_msync_flags(MS_SYNC | MS_INVALIDATE), Ok(()));
    }

    /// Specifying no synchronization mode is rejected.
    #[test]
    fn rejects_no_mode() {
        assert_eq!(validate_msync_flags(0), Err(ErrorCode::InvalidArgument));
        assert_eq!(validate_msync_flags(MS_INVALIDATE), Err(ErrorCode::InvalidArgument));
    }

    /// Specifying both synchronization modes is rejected.
    #[test]
    fn rejects_both_modes() {
        assert_eq!(validate_msync_flags(MS_ASYNC | MS_SYNC), Err(ErrorCode::InvalidArgument));
    }

    /// An unknown flag bit is rejected.
    #[test]
    fn rejects_unknown_bit() {
        let unknown: ::sysapi::ffi::c_int = MS_ASYNC | MS_INVALIDATE | MS_SYNC | 0x100;
        assert_eq!(validate_msync_flags(unknown), Err(ErrorCode::InvalidArgument));
    }
}
