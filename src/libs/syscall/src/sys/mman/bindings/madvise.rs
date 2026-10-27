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
use ::sysapi::sys_mman::memory_advice::{
    MADV_DONTNEED,
    MADV_FREE,
    MADV_NORMAL,
    MADV_RANDOM,
    MADV_SEQUENTIAL,
    MADV_WILLNEED,
};
#[cfg(feature = "syscall")]
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Validates the advice supplied to the `madvise()` binding.
///
/// Only the conventional advice values, `MADV_NORMAL`, `MADV_RANDOM`, `MADV_SEQUENTIAL`,
/// `MADV_WILLNEED`, `MADV_DONTNEED`, and `MADV_FREE`, are recognized. Any other value is rejected,
/// as Linux does with advice that it does not support, so that callers that probe for optional
/// advice can fall back to a conventional one.
///
/// # Parameters
///
/// - `advice`: Advice supplied by the caller.
///
/// # Returns
///
/// Returns `Ok(())` if `advice` is recognized, otherwise the `ErrorCode` to report via `errno`.
///
#[cfg(any(feature = "syscall", test))]
fn validate_madvise_advice(advice: c_int) -> Result<(), ErrorCode> {
    match advice {
        MADV_NORMAL | MADV_RANDOM | MADV_SEQUENTIAL | MADV_WILLNEED | MADV_DONTNEED | MADV_FREE => {
            Ok(())
        },
        _ => Err(ErrorCode::InvalidArgument),
    }
}

///
/// # Description
///
/// Advises the system about the expected use of the whole pages containing any part of the address
/// range that starts at `addr` and spans `length` bytes.
///
/// Advice is only a hint, and leaving the pages untouched is a valid response to every advice
/// value. Nanvix keeps every mapped page resident and never reads ahead, pages out, or reclaims
/// pages, so there is no paging behavior for advice to tune. This function therefore validates its
/// arguments and reports success without acting on the advice.
///
/// # Parameters
///
/// - `addr`: Start address of the region to advise on.
/// - `length`: Number of bytes to advise on.
/// - `advice`: Expected use of the region (`MADV_NORMAL`, `MADV_RANDOM`, `MADV_SEQUENTIAL`,
///   `MADV_WILLNEED`, `MADV_DONTNEED`, or `MADV_FREE`).
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error:
/// - `EINVAL`: `advice` is invalid, or `addr` is not a multiple of the page size.
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
/// - Advice is a no-op: pages are never read ahead, paged out, or reclaimed.
/// - `MADV_DONTNEED` and `MADV_FREE` do not discard page contents. Unlike on Linux, where private
///   anonymous pages read back as zeros after `MADV_DONTNEED`, the range keeps its contents.
/// - Only ranges mapped with `mmap()` are recognized as mapped. Advising on other memory, such as
///   the heap or the stack, fails with `ENOMEM`.
///
#[cfg(feature = "syscall")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn madvise(addr: *mut c_void, length: c_size_t, advice: c_int) -> c_int {
    if let Err(code) = validate_madvise_advice(advice) {
        unsafe {
            *__errno_location() = code.get();
        }
        return -1;
    }

    match super::validate_lock_range(addr as usize, length) {
        Ok(None) => 0,
        Ok(Some((addr, length))) => {
            let addr: VirtualAddress = VirtualAddress::from_raw_value(addr);
            match mman::madvise(addr, length) {
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
    use super::validate_madvise_advice;
    use ::sys::error::ErrorCode;
    use ::sysapi::{
        ffi::c_int,
        sys_mman::memory_advice::{
            MADV_DONTNEED,
            MADV_FREE,
            MADV_NORMAL,
            MADV_RANDOM,
            MADV_SEQUENTIAL,
            MADV_WILLNEED,
        },
    };

    /// Every conventional advice value is accepted.
    #[test]
    fn accepts_conventional_advice() {
        for advice in [
            MADV_NORMAL,
            MADV_RANDOM,
            MADV_SEQUENTIAL,
            MADV_WILLNEED,
            MADV_DONTNEED,
            MADV_FREE,
        ] {
            assert_eq!(validate_madvise_advice(advice), Ok(()), "advice={advice}");
        }
    }

    /// The unassigned values between `MADV_DONTNEED` and `MADV_FREE` are rejected.
    #[test]
    fn rejects_unassigned_advice() {
        for advice in (MADV_DONTNEED + 1)..MADV_FREE {
            assert_eq!(
                validate_madvise_advice(advice),
                Err(ErrorCode::InvalidArgument),
                "advice={advice}"
            );
        }
    }

    /// Values outside the range of conventional advice values are rejected.
    #[test]
    fn rejects_out_of_range_advice() {
        for advice in [c_int::MIN, -1, MADV_FREE + 1, c_int::MAX] {
            assert_eq!(
                validate_madvise_advice(advice),
                Err(ErrorCode::InvalidArgument),
                "advice={advice}"
            );
        }
    }
}
