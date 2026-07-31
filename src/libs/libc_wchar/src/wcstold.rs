// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    wchar_t::wchar_t,
    wcs_narrow::{
        to_narrow_alloc,
        NarrowString,
    },
};
#[cfg(all(
    target_arch = "aarch64",
    target_os = "nanvix",
    not(any(feature = "std", test))
))]
use ::libc_stdlib::binary128::{
    strto_binary128,
    Binary128,
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Converts the initial portion of the wide string `nptr` to a `long double` value.
///
/// The C prototype returns `long double` to match POSIX. On i686 and x86_64, this retains the
/// legacy double-precision conversion shared with `strtold`. AAPCS64 uses IEEE binary128, so the
/// AArch64 implementation narrows the wide input only for lexical scanning and converts it directly
/// to binary128 before returning it in `q0`.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg(not(all(
    any(
        target_arch = "x86_64",
        all(target_arch = "aarch64", target_os = "nanvix")
    ),
    not(any(feature = "std", test))
)))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstold(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    unsafe { wcstold_impl(nptr, endptr) }
}

// The x86_64 guest ABI returns `long double` differently from Rust's `f64`, so it uses this
// double-precision producer from an assembly trampoline. AArch64 has a separate direct binary128
// producer below.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nanvix_wcstold_f64(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    unsafe { wcstold_impl(nptr, endptr) }
}

// x86_64 guest: export `wcstold` as an assembly trampoline that computes the value at `double`
// precision (result in `xmm0`) and reloads it onto the x87 stack with `fldl`, so the `long double`
// return value is delivered in `st0`. `sub $8, %rsp` reserves an 8-byte spill slot and keeps `%rsp`
// 16-byte aligned at the `call`.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global wcstold",
    ".type wcstold, @function",
    "wcstold:",
    "    sub $8, %rsp",
    "    call __nanvix_wcstold_f64",
    "    movsd %xmm0, (%rsp)",
    "    fldl (%rsp)",
    "    add $8, %rsp",
    "    ret",
    options(att_syntax),
);

// AArch64 guest: a 16-byte C-compatible result is returned in X0/X1 and placed into Q0's lower
// and upper 64-bit lanes, reconstructing the IEEE-754 binary128 `long double` result.
#[cfg(all(
    target_arch = "aarch64",
    target_os = "nanvix",
    not(any(feature = "std", test))
))]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nanvix_wcstold_binary128(
    nptr: *const wchar_t,
    endptr: *mut *mut wchar_t,
) -> Binary128 {
    unsafe { wcstold_binary128_impl(nptr, endptr) }
}

#[cfg(all(
    target_arch = "aarch64",
    target_os = "nanvix",
    not(any(feature = "std", test))
))]
core::arch::global_asm!(
    ".global wcstold",
    ".type wcstold, @function",
    "wcstold:",
    "    stp x29, x30, [sp, #-16]!",
    "    mov x29, sp",
    "    bl __nanvix_wcstold_binary128",
    "    fmov d0, x0",
    "    ins v0.d[1], x1",
    "    ldp x29, x30, [sp], #16",
    "    ret",
);

/// Shared `double`-precision conversion backing `wcstold` on every target. Narrows the wide string
/// to its byte representation and delegates to `strtod`.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg(not(all(
    target_arch = "aarch64",
    target_os = "nanvix",
    not(any(feature = "std", test))
)))]
unsafe fn wcstold_impl(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    extern "C" {
        fn strtod(s: *const c_char, e: *mut *mut c_char) -> f64;
    }
    let narrow: NarrowString = match unsafe { to_narrow_alloc(nptr) } {
        Some(narrow) => narrow,
        None => {
            if !endptr.is_null() {
                unsafe { *endptr = nptr.cast_mut() };
            }
            return 0.0;
        },
    };
    let mut nend: *mut c_char = core::ptr::null_mut();
    let val: f64 = unsafe { strtod(narrow.as_ptr(), &mut nend) };
    if !endptr.is_null() {
        let consumed: usize = (nend as usize) - (narrow.as_ptr() as usize);
        unsafe { *endptr = nptr.add(consumed).cast_mut() };
    }
    val
}

/// Shared binary128 conversion backing AArch64 `wcstold`.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg(all(
    target_arch = "aarch64",
    target_os = "nanvix",
    not(any(feature = "std", test))
))]
unsafe fn wcstold_binary128_impl(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> Binary128 {
    let narrow: NarrowString = match unsafe { to_narrow_alloc(nptr) } {
        Some(narrow) => narrow,
        None => {
            if !endptr.is_null() {
                unsafe { *endptr = nptr.cast_mut() };
            }
            return Binary128 { low: 0, high: 0 };
        },
    };
    let mut narrow_end: *mut c_char = core::ptr::null_mut();
    let value: Binary128 = unsafe { strto_binary128(narrow.as_ptr(), &mut narrow_end) };
    if !endptr.is_null() {
        let consumed: usize = (narrow_end as usize) - (narrow.as_ptr() as usize);
        unsafe { *endptr = nptr.add(consumed).cast_mut() };
    }
    value
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcstold_simple() {
        // "3.50" parses to 3.5 and leaves the end pointer at the terminator.
        let s: [wchar_t; 5] = [0x33, 0x2E, 0x35, 0x30, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: f64 = unsafe { wcstold(s.as_ptr(), &mut end) };
        assert!((v - 3.5).abs() < 1e-9);
        assert_eq!(unsafe { *end }, 0);
    }

    #[test]
    fn test_wcstold_partial() {
        // "2.5x" parses 2.5 and stops at 'x'.
        let s: [wchar_t; 5] = [0x32, 0x2E, 0x35, 0x78, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: f64 = unsafe { wcstold(s.as_ptr(), &mut end) };
        assert!((v - 2.5).abs() < 1e-9);
        assert_eq!(unsafe { *end }, 0x78);
    }

    #[test]
    fn test_wcstold_null_endptr() {
        // A null end pointer is accepted.
        let s: [wchar_t; 3] = [0x31, 0x30, 0];
        let v: f64 = unsafe { wcstold(s.as_ptr(), core::ptr::null_mut()) };
        assert!((v - 10.0).abs() < 1e-9);
    }
}
