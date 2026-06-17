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

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::setjmp;
    use crate::jmp_buf::jmp_buf;
    use ::sysapi::ffi::c_int;

    // On the host, setjmp is a non-functional stub that always reports a direct (zero) return. The
    // real non-local control flow only exists in the guest assembly build, which cannot run under
    // the host test harness.
    #[test]
    fn setjmp_stub_reports_direct_return() {
        let mut buf: jmp_buf = jmp_buf { regs: [0; 6] };
        let result: c_int = unsafe { setjmp(&mut buf) };
        assert_eq!(result, 0);
    }
}
