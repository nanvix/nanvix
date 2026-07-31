// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Saves the calling environment in `env` for later use by `longjmp`.
///
/// # Parameters
///
/// - `env`: Pointer to a `jmp_buf` in which the environment is saved.
///
/// # Return Value
///
/// Returns zero when called directly. When returning from a `longjmp` call, returns the value
/// passed to `longjmp` (or 1 if that value was 0).
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer.
///
// Host/std/test builds: provide a non-exported stub. Mirroring the libc_assert convention, the
// C-ABI symbol is exported only in guest (no_std) builds, so `no_mangle` is deliberately omitted
// here to avoid colliding with the host C library's `setjmp` when this crate is unit-tested on the
// host.
#[cfg(any(feature = "std", test))]
pub unsafe extern "C" fn setjmp(_env: *mut crate::jmp_buf::jmp_buf) -> ::sysapi::ffi::c_int {
    0
}

// Guest (no_std) build: the real implementation is x86-32 assembly. The assembler's `.global
// setjmp` directive exports the C symbol directly and is the equivalent of `no_mangle`, applied
// only in non-std builds (see libc_assert).
#[cfg(all(target_arch = "x86", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global setjmp",
    ".type setjmp, @function",
    "setjmp:",
    "    mov 4(%esp), %eax",  // eax = pointer to jmp_buf
    "    mov %ebx, 0(%eax)",  // save EBX
    "    mov %esi, 4(%eax)",  // save ESI
    "    mov %edi, 8(%eax)",  // save EDI
    "    mov %ebp, 12(%eax)", // save EBP
    "    lea 4(%esp), %ecx",  // compute original ESP (before call pushed return addr)
    "    mov %ecx, 16(%eax)", // save ESP
    "    mov (%esp), %ecx",   // get return address
    "    mov %ecx, 20(%eax)", // save EIP (return addr)
    "    xor %eax, %eax",     // return 0
    "    ret",
    options(att_syntax),
);

// Guest (no_std) build: the real implementation is x86-64 assembly. Per the System V AMD64 ABI the
// pointer argument arrives in RDI and the callee-saved registers are RBX, RBP, R12-R15, and RSP;
// these plus the return address (RIP) are the context restored by longjmp. The assembler's `.global
// setjmp` directive exports the C symbol directly and is the equivalent of `no_mangle`, applied only
// in non-std builds (see libc_assert).
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global setjmp",
    ".type setjmp, @function",
    "setjmp:",
    "    mov %rbx, 0(%rdi)",  // save RBX
    "    mov %rbp, 8(%rdi)",  // save RBP
    "    mov %r12, 16(%rdi)", // save R12
    "    mov %r13, 24(%rdi)", // save R13
    "    mov %r14, 32(%rdi)", // save R14
    "    mov %r15, 40(%rdi)", // save R15
    "    lea 8(%rsp), %rax",  // compute original RSP (before call pushed return addr)
    "    mov %rax, 48(%rdi)", // save RSP
    "    mov (%rsp), %rax",   // get return address
    "    mov %rax, 56(%rdi)", // save RIP (return addr)
    "    xor %eax, %eax",     // return 0
    "    ret",
    options(att_syntax),
);

// Guest (no_std) build: AAPCS64 preserves X19-X30, SP, and the low 64 bits of V8-V15.
#[cfg(all(target_arch = "aarch64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global setjmp",
    ".type setjmp, @function",
    "setjmp:",
    "    stp x19, x20, [x0, #0]",
    "    stp x21, x22, [x0, #16]",
    "    stp x23, x24, [x0, #32]",
    "    stp x25, x26, [x0, #48]",
    "    stp x27, x28, [x0, #64]",
    "    stp x29, x30, [x0, #80]",
    "    mov x1, sp",
    "    str x1, [x0, #96]",
    "    stp d8, d9, [x0, #104]",
    "    stp d10, d11, [x0, #120]",
    "    stp d12, d13, [x0, #136]",
    "    stp d14, d15, [x0, #152]",
    "    mov w0, wzr",
    "    ret",
);

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::setjmp;
    use crate::jmp_buf::{
        jmp_buf,
        JMP_BUF_REGS,
    };
    use ::sysapi::ffi::c_int;

    // On the host, setjmp is a non-functional stub that always reports a direct (zero) return. The
    // real non-local control flow only exists in the guest assembly build, which cannot run under
    // the host test harness.
    #[test]
    fn setjmp_stub_reports_direct_return() {
        let mut buf: jmp_buf = jmp_buf {
            regs: [0; JMP_BUF_REGS],
        };
        let result: c_int = unsafe { setjmp(&mut buf) };
        assert_eq!(result, 0);
    }
}
