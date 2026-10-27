// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "syscall")]
use crate::errno::__errno_location;
#[cfg(any(feature = "syscall", test))]
use ::sys::error::ErrorCode;
#[cfg(any(feature = "syscall", test))]
use ::sysapi::ffi::c_int;
#[cfg(any(feature = "syscall", test))]
use ::sysapi::sys_mman::mlockall_flags::{
    MCL_CURRENT,
    MCL_FUTURE,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Validates the flags supplied to the `mlockall()` binding.
///
/// POSIX requires `flags` to request at least one of `MCL_CURRENT` and `MCL_FUTURE`, and to include
/// no flag that the implementation does not support. Zero and any bit other than these two flags
/// are therefore rejected. Rejecting unknown flags, such as the Linux-specific `MCL_ONFAULT`, also
/// lets callers that probe for optional flags fall back to the POSIX ones.
///
/// # Parameters
///
/// - `flags`: Locking flags supplied by the caller.
///
/// # Returns
///
/// Returns `Ok(())` if `flags` is a valid combination, otherwise the `ErrorCode` to report via
/// `errno`.
///
#[cfg(any(feature = "syscall", test))]
fn validate_mlockall_flags(flags: c_int) -> Result<(), ErrorCode> {
    // Reject any bit outside the set of recognized flags.
    const KNOWN: c_int = MCL_CURRENT | MCL_FUTURE;
    if flags & !KNOWN != 0 {
        return Err(ErrorCode::InvalidArgument);
    }

    // At least one kind of locking must be requested.
    if flags == 0 {
        return Err(ErrorCode::InvalidArgument);
    }

    Ok(())
}

///
/// # Description
///
/// Locks into memory the pages mapped into the address space of the calling process, guaranteeing
/// that they remain memory-resident.
///
/// `MCL_CURRENT` locks the pages that are currently mapped, and `MCL_FUTURE` locks the pages that
/// become mapped later on, for instance as the heap or the stack grows or by calls to `mmap()`.
///
/// Nanvix never swaps or pages out memory, so every page is resident from the moment it becomes
/// mapped until it is unmapped. Both kinds of locking therefore always hold, and this function only
/// validates its arguments.
///
/// # Parameters
///
/// - `flags`: Bitwise OR of `MCL_CURRENT` and `MCL_FUTURE`, selecting the pages to lock.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error:
/// - `EINVAL`: `flags` is zero or includes unknown flags.
///
/// # Safety
///
/// This function is unsafe because it may modify `errno`. It is safe to call provided that access
/// to `errno` is synchronized with other threads that may modify it.
///
/// # Known Limitations (Nanvix)
///
/// - Locking is a no-op: mapped pages are always resident.
/// - The Linux-specific `MCL_ONFAULT` flag is not supported and is rejected with `EINVAL`.
///
#[cfg(feature = "syscall")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mlockall(flags: c_int) -> c_int {
    match validate_mlockall_flags(flags) {
        Ok(()) => 0,
        Err(code) => {
            ::syslog::warn!("mlockall(): invalid flags (flags={flags:#x})");
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
    use super::validate_mlockall_flags;
    use ::sys::error::ErrorCode;
    use ::sysapi::{
        ffi::c_int,
        sys_mman::mlockall_flags::{
            MCL_CURRENT,
            MCL_FUTURE,
        },
    };

    /// A lone `MCL_CURRENT` flag is accepted.
    #[test]
    fn accepts_mcl_current() {
        assert_eq!(validate_mlockall_flags(MCL_CURRENT), Ok(()));
    }

    /// A lone `MCL_FUTURE` flag is accepted.
    #[test]
    fn accepts_mcl_future() {
        assert_eq!(validate_mlockall_flags(MCL_FUTURE), Ok(()));
    }

    /// Both flags combined are accepted.
    #[test]
    fn accepts_both_flags() {
        assert_eq!(validate_mlockall_flags(MCL_CURRENT | MCL_FUTURE), Ok(()));
    }

    /// Requesting no kind of locking is rejected.
    #[test]
    fn rejects_no_flags() {
        assert_eq!(validate_mlockall_flags(0), Err(ErrorCode::InvalidArgument));
    }

    /// Every bit other than the two flags is rejected, alone or combined with the flags.
    #[test]
    fn rejects_unknown_bits() {
        let known: c_int = MCL_CURRENT | MCL_FUTURE;
        for bit in 0..c_int::BITS {
            let unknown: c_int = 1 << bit;
            if unknown & known != 0 {
                continue;
            }

            for base in [0, MCL_CURRENT, MCL_FUTURE, known] {
                let flags: c_int = base | unknown;
                assert_eq!(
                    validate_mlockall_flags(flags),
                    Err(ErrorCode::InvalidArgument),
                    "flags={flags:#x}"
                );
            }
        }
    }
}
