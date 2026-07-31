// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Guest helper called by the architecture trampolines before saving the register context. Calling
// through sigprocmask keeps signal-mask policy in the existing libc implementation.
#[cfg(not(any(feature = "std", test)))]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nanvix_sigsetjmp_save_mask(
    env: *mut crate::sigjmp_buf::sigjmp_buf,
    savemask: ::sysapi::ffi::c_int,
) {
    use ::sysapi::{
        ffi::c_int,
        signal::{
            sigset_t,
            SIG_SETMASK,
        },
    };

    extern "C" {
        fn sigprocmask(how: c_int, set: *const sigset_t, oldset: *mut sigset_t) -> c_int;
    }

    unsafe {
        (*env).savemask = 0;
    }

    if savemask != 0 {
        let result: c_int =
            unsafe { sigprocmask(SIG_SETMASK, ::core::ptr::null(), &raw mut (*env).sigmask) };
        if result == 0 {
            unsafe {
                (*env).savemask = savemask;
            }
        }
    }
}

///
/// # Description
///
/// Saves the calling environment in `env` for later use by `siglongjmp`.
///
/// This is equivalent to `setjmp`, except that when `savemask` is nonzero `sigsetjmp` also records
/// that the signal mask should be saved as part of the calling environment.
///
/// # Parameters
///
/// - `env`: Pointer to a `sigjmp_buf` in which the environment is saved.
/// - `savemask`: If nonzero, the current signal mask is saved in `env` and restored by a matching
///   `siglongjmp`.
///
/// # Return Value
///
/// Returns zero when called directly. When returning from a `siglongjmp` call, returns the value
/// passed to `siglongjmp` (or 1 if that value was 0).
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer.
///
// Host/std/test builds: provide a non-exported stub. Mirroring the libc_assert convention, the
// C-ABI symbol is exported only in guest (no_std) builds, so `no_mangle` is deliberately omitted
// here to avoid colliding with the host C library's `sigsetjmp` when this crate is unit-tested on
// the host.
#[cfg(any(feature = "std", test))]
pub unsafe extern "C" fn sigsetjmp(
    _env: *mut crate::sigjmp_buf::sigjmp_buf,
    _savemask: ::sysapi::ffi::c_int,
) -> ::sysapi::ffi::c_int {
    0
}

// Guest (no_std) build: save the requested signal mask through the libc API, restore the original
// stack shape, and tail-call setjmp. The helper follows cdecl and preserves the caller's
// callee-saved registers, so setjmp still captures the original execution context.
#[cfg(all(target_arch = "x86", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global sigsetjmp",
    ".type sigsetjmp, @function",
    "sigsetjmp:",
    "    mov 4(%esp), %eax",
    "    mov 8(%esp), %ecx",
    "    sub $4, %esp",
    "    push %ecx",
    "    push %eax",
    "    call __nanvix_sigsetjmp_save_mask",
    "    add $12, %esp",
    "    jmp setjmp",
    options(att_syntax),
);

// Guest (no_std) build: save RDI across the helper call, then restore the entry stack pointer and
// tail-call setjmp so it captures the caller's context rather than this trampoline's context.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global sigsetjmp",
    ".type sigsetjmp, @function",
    "sigsetjmp:",
    "    sub $8, %rsp",
    "    mov %rdi, (%rsp)",
    "    call __nanvix_sigsetjmp_save_mask",
    "    mov (%rsp), %rdi",
    "    add $8, %rsp",
    "    jmp setjmp",
    options(att_syntax),
);

// Guest (no_std) build: preserve the caller's link register and environment pointer across the
// helper call, then tail-call setjmp with the original stack and link register.
#[cfg(all(target_arch = "aarch64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global sigsetjmp",
    ".type sigsetjmp, @function",
    "sigsetjmp:",
    "    stp x0, x30, [sp, #-16]!",
    "    bl __nanvix_sigsetjmp_save_mask",
    "    ldp x0, x30, [sp], #16",
    "    b setjmp",
);

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::sigsetjmp;
    use crate::sigjmp_buf::sigjmp_buf;
    use ::sysapi::ffi::c_int;

    // On the host, sigsetjmp is a non-functional stub that always reports a direct (zero) return.
    // The real non-local control flow only exists in the guest assembly build, which cannot run
    // under the host test harness.
    #[test]
    fn sigsetjmp_stub_reports_direct_return() {
        let mut buf: sigjmp_buf = sigjmp_buf {
            regs: [0; crate::jmp_buf::JMP_BUF_REGS],
            savemask: 0,
            sigmask: 0,
        };
        let result: c_int = unsafe { sigsetjmp(&mut buf, 1) };
        assert_eq!(result, 0);
    }
}
