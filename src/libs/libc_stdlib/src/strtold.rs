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
/// The C prototype returns `long double` to match POSIX. The conversion is computed at `double`
/// precision and delegated to [`crate::strtod`]; the result is then returned in whatever register
/// class the C ABI reads a `long double` from. On the i686 guest the cdecl convention returns
/// `float`, `double`, and `long double` alike in the x87 `st0`, so the `f64` result is promoted to
/// the 80-bit extended representation on return with no extra work. The x86_64 System V ABI instead
/// returns `double` in `xmm0` but `long double` in `st0`, so on that target `strtold` is exported as
/// a small assembly trampoline (below) that reloads the `xmm0` result onto the x87 stack.
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
#[cfg(not(all(target_arch = "x86_64", not(any(feature = "std", test)))))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtold(nptr: *const c_char, endptr: *mut *mut c_char) -> f64 {
    crate::strtod(nptr, endptr)
}

// x86_64 guest: `double`-precision producer used by the assembly trampoline below. Kept as an
// internal symbol (double-underscore, reserved for the implementation) so the public `strtold` can
// convert its `xmm0` result into the `st0` return value the System V AMD64 ABI mandates for
// `long double`.
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
