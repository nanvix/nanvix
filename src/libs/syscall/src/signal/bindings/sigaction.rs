// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::signal::sigaction_t;
use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Compile-Time ABI Assertions
//==================================================================================================

// `sigaction_t` (the user-space C ABI view) and `sys::pm::SigAction` (the kernel-call view) are not
// textually identical — `sa_sigaction` is `Option<extern "C" fn(...)>` here but `usize` in
// `SigAction` (pointer-sized via the null-pointer optimization). The pointer casts in [`sigaction`]
// are only sound while the two stay ABI-compatible, so guard the size, alignment, and field offsets
// at compile time; any future divergence fails the build instead of silently corrupting the ABI.
const _: () = {
    assert!(
        ::core::mem::size_of::<sigaction_t>() == ::core::mem::size_of::<::sys::pm::SigAction>()
    );
    assert!(
        ::core::mem::align_of::<sigaction_t>() == ::core::mem::align_of::<::sys::pm::SigAction>()
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_handler)
            == ::core::mem::offset_of!(::sys::pm::SigAction, sa_handler)
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_mask)
            == ::core::mem::offset_of!(::sys::pm::SigAction, sa_mask)
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_flags)
            == ::core::mem::offset_of!(::sys::pm::SigAction, sa_flags)
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_sigaction)
            == ::core::mem::offset_of!(::sys::pm::SigAction, sa_sigaction)
    );
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// `sigaction` — installs and/or queries the disposition of a signal via the `sigaction()` kernel
/// call.
///
/// `sigaction_t` and `sys::pm::SigAction` are ABI-compatible (same `repr(C)` size, alignment, and
/// field offsets, asserted at compile time above), so the action pointers are reinterpreted across
/// the kernel-call boundary without translation.
#[unsafe(no_mangle)]
#[trace_syscall]
pub extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction_t,
    oldact: *mut sigaction_t,
) -> c_int {
    match unsafe {
        ::sys::kcall::pm::__kcall_sigaction(
            signum,
            act as *const ::sys::pm::SigAction,
            oldact as *mut ::sys::pm::SigAction,
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
