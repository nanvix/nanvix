// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Guest helper called before restoring the register context. siglongjmp cannot report a
// sigprocmask failure, so it attempts the restore and always continues with the non-local jump.
#[cfg(not(any(feature = "std", test)))]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nanvix_siglongjmp_restore_mask(env: *const crate::sigjmp_buf::sigjmp_buf) {
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

    if unsafe { (*env).savemask } != 0 {
        let _: c_int =
            unsafe { sigprocmask(SIG_SETMASK, &raw const (*env).sigmask, ::core::ptr::null_mut()) };
    }
}

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

// Guest (no_std) build: restore the saved mask through the libc API, recover the original entry
// stack, and tail-call longjmp. The original env/value arguments remain on the cdecl stack.
#[cfg(all(target_arch = "x86", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global siglongjmp",
    ".type siglongjmp, @function",
    "siglongjmp:",
    "    mov 4(%esp), %eax",
    "    sub $8, %esp",
    "    push %eax",
    "    call __nanvix_siglongjmp_restore_mask",
    "    add $12, %esp",
    "    jmp longjmp",
    options(att_syntax),
);

// Guest (no_std) build: preserve env/value across the helper call, restore the entry stack, and
// tail-call longjmp, which does not return.
#[cfg(all(target_arch = "aarch64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global siglongjmp",
    ".type siglongjmp, @function",
    "siglongjmp:",
    "    stp x0, x1, [sp, #-16]!",
    "    bl __nanvix_siglongjmp_restore_mask",
    "    ldp x0, x1, [sp], #16",
    "    b longjmp",
);

// Guest (no_std) build: spill env/value in caller-owned stack space across the helper call, restore
// the entry stack, and tail-call longjmp.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global siglongjmp",
    ".type siglongjmp, @function",
    "siglongjmp:",
    "    sub $24, %rsp",
    "    mov %rdi, (%rsp)",
    "    mov %esi, 8(%rsp)",
    "    call __nanvix_siglongjmp_restore_mask",
    "    mov (%rsp), %rdi",
    "    mov 8(%rsp), %esi",
    "    add $24, %rsp",
    "    jmp longjmp",
    options(att_syntax),
);
