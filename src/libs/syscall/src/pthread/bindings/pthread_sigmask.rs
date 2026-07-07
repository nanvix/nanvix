// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::signal::sigset_t;
use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Compile-Time ABI Assertions
//==================================================================================================

// The user-space `sigset_t` and the kernel-call `sys::pm::SigSet` are both 64-bit blocked-signal
// bitmasks, so the pointer casts below are only sound while the two stay ABI-compatible. Guard
// their size and alignment at compile time; any future divergence fails the build instead of
// silently corrupting the ABI.
::static_assert::assert_eq_size!(sigset_t, ::core::mem::size_of::<::sys::pm::SigSet>());
::static_assert::assert_eq_align!(sigset_t, ::core::mem::align_of::<::sys::pm::SigSet>());

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Examines and/or changes the calling thread's blocked-signal mask.
///
/// `pthread_sigmask()` has the same semantics as `sigprocmask()`, differing only in how it reports
/// errors: it returns the error number directly instead of setting `errno`. Nanvix keeps the
/// blocked-signal mask per thread in the kernel, so both calls funnel through the same
/// `sigprocmask()` kernel call.
///
/// # Parameters
///
/// - `how`: One of `SIG_BLOCK`, `SIG_UNBLOCK`, or `SIG_SETMASK`; ignored when `set` is null.
/// - `set`: New signals to apply per `how`, or null to leave the mask unchanged.
/// - `oldset`: Receives the previous mask, or null if not wanted.
///
/// # Returns
///
/// Zero on success, or a positive error number on failure.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `set` and `oldset`, which must
/// each be either null or a valid, properly aligned pointer to a `sigset_t`.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/pthread_sigmask.html>
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_sigmask(
    how: c_int,
    set: *const sigset_t,
    oldset: *mut sigset_t,
) -> c_int {
    match unsafe {
        ::sys::kcall::pm::__kcall_sigprocmask(
            how,
            set as *const ::sys::pm::SigSet,
            oldset as *mut ::sys::pm::SigSet,
        )
    } {
        Ok(()) => 0,
        Err(error) => error.code.get(),
    }
}
