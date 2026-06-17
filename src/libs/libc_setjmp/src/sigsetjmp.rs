// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

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

// Guest (no_std) build: the real implementation is x86-32 assembly. POSIX defines sigsetjmp to be
// equivalent to setjmp (except for the signal mask), so it records the savemask flag and then
// tail-calls setjmp to capture the register context. Reusing setjmp guarantees the two cannot
// diverge. The guest does not yet maintain a signal mask (pthread_sigmask is a no-op), so there is
// nothing to save when savemask is nonzero; the flag is recorded so the behavior becomes correct
// automatically once a signal mask is implemented.
#[cfg(all(target_arch = "x86", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global sigsetjmp",
    ".type sigsetjmp, @function",
    "sigsetjmp:",
    "    mov 4(%esp), %eax",  // eax = pointer to sigjmp_buf
    "    mov 8(%esp), %ecx",  // ecx = savemask
    "    mov %ecx, 24(%eax)", // save savemask flag
    "    jmp setjmp",         // tail-call setjmp: saves the register context and returns 0
    options(att_syntax),
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
            regs: [0; 6],
            savemask: 0,
        };
        let result: c_int = unsafe { sigsetjmp(&mut buf, 1) };
        assert_eq!(result, 0);
    }
}
