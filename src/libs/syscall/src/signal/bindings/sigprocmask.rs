// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::signal::sigset_t;
use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Compile-Time ABI Assertions
//==================================================================================================

// `sigset_t` (the user-space C ABI view) and `sys::pm::SigSet` (the kernel-call view) are both
// 64-bit blocked-signal bitmasks. The pointer casts in [`sigprocmask`] are only sound while the
// two stay ABI-compatible, so guard their size and alignment at compile time; any future
// divergence fails the build instead of silently corrupting the ABI.
const _: () = {
    assert!(::core::mem::size_of::<sigset_t>() == ::core::mem::size_of::<::sys::pm::SigSet>());
    assert!(::core::mem::align_of::<sigset_t>() == ::core::mem::align_of::<::sys::pm::SigSet>());
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Examines and/or changes the calling process's blocked-signal mask via the `sigprocmask()`
/// kernel call.
///
/// `sigset_t` and `sys::pm::SigSet` are ABI-compatible (same size and alignment, asserted at
/// compile time above), so the mask pointers are reinterpreted across the kernel-call boundary
/// without translation.
///
/// # Parameters
///
/// - `how`: How to combine `set` with the current mask (`SIG_BLOCK`, `SIG_UNBLOCK`, or
///   `SIG_SETMASK`); ignored when `set` is null.
/// - `set`: Pointer to the signals to apply per `how`, or null to leave the mask unchanged.
/// - `oldset`: Pointer that receives the previous mask, or null if not wanted.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, `-1` is returned and `errno` is set
/// to indicate the error.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub extern "C" fn sigprocmask(how: c_int, set: *const sigset_t, oldset: *mut sigset_t) -> c_int {
    match unsafe {
        ::sys::kcall::pm::__kcall_sigprocmask(
            how,
            set as *const ::sys::pm::SigSet,
            oldset as *mut ::sys::pm::SigSet,
        )
    } {
        Ok(()) => 0,
        Err(error) => {
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}
