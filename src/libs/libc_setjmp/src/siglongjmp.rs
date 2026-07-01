// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Restores the environment saved by `sigsetjmp` and causes `sigsetjmp` to return `val`.
///
/// This is equivalent to `longjmp`, except that the saved signal mask is restored if and only if
/// `env` was initialized by a `sigsetjmp` call with a nonzero `savemask` argument.
///
/// # Parameters
///
/// - `env`: Pointer to a `sigjmp_buf` previously filled by `sigsetjmp`.
/// - `val`: Value that `sigsetjmp` will appear to return. If `val` is 0, `sigsetjmp` returns 1
///   instead.
///
/// # Return Value
///
/// This function does not return.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer and performs a non-local jump.
///
// Host/std/test builds: provide a non-exported stub. Mirroring the libc_assert convention, the
// C-ABI symbol is exported only in guest (no_std) builds, so `no_mangle` is deliberately omitted
// here to avoid colliding with the host C library's `siglongjmp` when this crate is built on the
// host.
#[cfg(any(feature = "std", test))]
pub unsafe extern "C" fn siglongjmp(
    _env: *mut crate::sigjmp_buf::sigjmp_buf,
    _val: ::sysapi::ffi::c_int,
) -> ! {
    ::std::process::abort()
}

// Guest (no_std) build: the real implementation is x86-32 assembly. POSIX defines siglongjmp to be
// equivalent to longjmp (except for the signal mask), so it tail-calls longjmp to restore the
// register context and resume. Reusing longjmp guarantees the two cannot diverge. A saved signal
// mask would be restored here when env->savemask is nonzero, but the guest does not yet maintain a
// signal mask (pthread_sigmask is a no-op), so there is nothing to restore.
#[cfg(all(target_arch = "x86", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global siglongjmp",
    ".type siglongjmp, @function",
    "siglongjmp:",
    "    jmp longjmp", // tail-call longjmp: restores the register context and resumes
    options(att_syntax),
);

// Guest (no_std) build: the real implementation is x86-64 assembly. POSIX defines siglongjmp to be
// equivalent to longjmp (except for the signal mask), so it tail-calls longjmp to restore the
// register context and resume. Reusing longjmp guarantees the two cannot diverge. A saved signal
// mask would be restored here when env->savemask is nonzero, but the guest does not yet maintain a
// signal mask (pthread_sigmask is a no-op), so there is nothing to restore.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global siglongjmp",
    ".type siglongjmp, @function",
    "siglongjmp:",
    "    jmp longjmp", // tail-call longjmp: restores the register context and resumes
    options(att_syntax),
);
