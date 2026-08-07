// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to a floating-point value.
///
/// The C prototype returns `long double` to match POSIX. On the i686 guest the cdecl convention
/// returns `float`, `double`, and `long double` alike in the x87 `st0`, so this legacy
/// implementation delegates to [`crate::strtod`]. The x86_64 System V ABI returns `long double` in
/// `st0`, so its legacy double-precision implementation uses the assembly trampoline below. AAPCS64
/// instead uses IEEE binary128: its trampoline calls the direct integer-based converter and moves
/// its binary128 result into `q0`, without narrowing finite input through `f64`.
///
/// # Parameters
///
/// - `nptr`: Pointer to the null-terminated string to be converted.
/// - `endptr`: If not null, receives a pointer to the first character not converted.
///
/// # Returns
///
/// The converted floating-point value.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers through [`crate::strtod`].
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtod.html
///
#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    not(any(feature = "std", test))
)))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtold(nptr: *const c_char, endptr: *mut *mut c_char) -> f64 {
    crate::strtod(nptr, endptr)
}

// The x86_64 guest ABI returns `long double` differently from Rust's `f64`, so it uses this
// double-precision producer from an assembly trampoline. AArch64 has a separate direct binary128
// producer below.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nanvix_strtold_f64(nptr: *const c_char, endptr: *mut *mut c_char) -> f64 {
    crate::strtod(nptr, endptr)
}

// x86_64 guest: export `strtold` as an assembly trampoline that computes the value at `double`
// precision (result in `xmm0`) and reloads it onto the x87 stack with `fldl`, so the `long double`
// return value is delivered in `st0` as the ABI requires. The `.global strtold` directive exports
// the C symbol directly (the equivalent of `no_mangle`), mirroring the libc_setjmp convention.
// `sub $8, %rsp` reserves an 8-byte spill slot and keeps `%rsp` 16-byte aligned at the `call`.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global strtold",
    ".type strtold, @function",
    "strtold:",
    "    sub $8, %rsp",
    "    call __nanvix_strtold_f64",
    "    movsd %xmm0, (%rsp)",
    "    fldl (%rsp)",
    "    add $8, %rsp",
    "    ret",
    options(att_syntax),
);

// AArch64 guest: use a C-compatible pair of 64-bit words to transport the direct binary128 bits.
// AAPCS64 returns a 16-byte composite in X0/X1. The words must be placed in the lower and upper
// 64-bit lanes of Q0; D1 is the lower lane of a separate Q1 register.
#[cfg(all(target_arch = "aarch64", not(any(feature = "std", test))))]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nanvix_strtold_binary128(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
) -> crate::binary128::Binary128 {
    unsafe { crate::binary128::strto_binary128(nptr, endptr) }
}

#[cfg(all(target_arch = "aarch64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global strtold",
    ".type strtold, @function",
    "strtold:",
    "    stp x29, x30, [sp, #-16]!",
    "    mov x29, sp",
    "    bl __nanvix_strtold_binary128",
    "    fmov d0, x0",
    "    ins v0.d[1], x1",
    "    ldp x29, x30, [sp], #16",
    "    ret",
);

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::strtold;
    use ::sysapi::ffi::c_char;

    #[test]
    fn delegates_to_strtod() {
        let s = b"0x1.8p+1\0";
        let value: f64 = unsafe { strtold(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert_eq!(value, 3.0);
    }
}
