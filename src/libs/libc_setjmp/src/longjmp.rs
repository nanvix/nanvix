// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Restores the environment saved by `setjmp` and causes `setjmp` to return `val`.
///
/// # Parameters
///
/// - `env`: Pointer to a `jmp_buf` previously filled by `setjmp`.
/// - `val`: Value that `setjmp` will appear to return. If `val` is 0, `setjmp` returns 1 instead.
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
// here to avoid colliding with the host C library's `longjmp` when this crate is built on the host.
#[cfg(any(feature = "std", test))]
pub unsafe extern "C" fn longjmp(
    _env: *mut crate::jmp_buf::jmp_buf,
    _val: ::sysapi::ffi::c_int,
) -> ! {
    ::std::process::abort()
}

// Guest (no_std) build: the real implementation is x86-32 assembly. The assembler's `.global
// longjmp` directive exports the C symbol directly and is the equivalent of `no_mangle`, applied
// only in non-std builds (see libc_assert).
#[cfg(all(target_arch = "x86", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global longjmp",
    ".type longjmp, @function",
    "longjmp:",
    "    mov 4(%esp), %edx", // edx = pointer to jmp_buf
    "    mov 8(%esp), %eax", // eax = return value
    "    test %eax, %eax",
    "    jnz 1f",
    "    inc %eax", // if val == 0, set to 1
    "1:",
    "    mov 0(%edx), %ebx",  // restore EBX
    "    mov 4(%edx), %esi",  // restore ESI
    "    mov 8(%edx), %edi",  // restore EDI
    "    mov 12(%edx), %ebp", // restore EBP
    "    mov 16(%edx), %esp", // restore ESP
    "    jmp *20(%edx)",      // jump to saved EIP
    options(att_syntax),
);

// Guest (no_std) build: the real implementation is x86-64 assembly. Per the System V AMD64 ABI the
// jmp_buf pointer arrives in RDI and the return value in ESI; it restores the callee-saved registers
// (RBX, RBP, R12-R15, RSP) and jumps to the saved RIP. The assembler's `.global longjmp` directive
// exports the C symbol directly and is the equivalent of `no_mangle`, applied only in non-std builds
// (see libc_assert).
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global longjmp",
    ".type longjmp, @function",
    "longjmp:",
    "    mov %esi, %eax", // eax = return value (val is a 32-bit c_int)
    "    test %eax, %eax",
    "    jnz 1f",
    "    inc %eax", // if val == 0, set to 1
    "1:",
    "    mov 0(%rdi), %rbx",  // restore RBX
    "    mov 8(%rdi), %rbp",  // restore RBP
    "    mov 16(%rdi), %r12", // restore R12
    "    mov 24(%rdi), %r13", // restore R13
    "    mov 32(%rdi), %r14", // restore R14
    "    mov 40(%rdi), %r15", // restore R15
    "    mov 48(%rdi), %rsp", // restore RSP
    "    jmp *56(%rdi)",      // jump to saved RIP
    options(att_syntax),
);
