// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    strtod::strtod,
    strtof::strtof,
    strtol::strtol,
    strtoll::strtoll,
    strtoul::strtoul,
    strtoull::strtoull,
};
// `strtold` backs only the non-x86_64-guest `strtold_l`; on the x86_64 guest `strtold_l` is an
// assembly tail-call to the `strtold` symbol and needs no Rust import.
#[cfg(not(all(target_arch = "x86_64", not(any(feature = "std", test)))))]
use crate::strtold::strtold;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_long,
    c_longlong,
    c_ulong,
    c_ulonglong,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Nanvix supports only the C/POSIX locale, so each `*_l` function ignores its `locale_t` argument
// and delegates to its non-`_l` counterpart.

/// Converts the initial portion of a string to a `double` using the C/POSIX locale.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtod_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    _locale: *mut c_void,
) -> f64 {
    unsafe { strtod(nptr, endptr) }
}

/// Converts the initial portion of a string to a `float` using the C/POSIX locale.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtof_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    _locale: *mut c_void,
) -> f32 {
    unsafe { strtof(nptr, endptr) }
}

/// Converts the initial portion of a string to a `long double` using the C/POSIX locale.
///
/// Delegates to [`strtold`]. On the x86_64 guest the `long double` return value must come back in
/// the x87 `st0` (see `strtold`), so this is exported there as an assembly tail-call to `strtold`
/// rather than a Rust wrapper, whose `-> f64` return would use `xmm0`.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg(not(all(target_arch = "x86_64", not(any(feature = "std", test)))))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtold_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    _locale: *mut c_void,
) -> f64 {
    unsafe { strtold(nptr, endptr) }
}

// x86_64 guest: tail-call `strtold` so its `st0` (long double) return propagates unchanged. The
// C/POSIX-only locale argument in `%rdx` is ignored; `%rdi`/`%rsi` already hold `nptr`/`endptr`.
#[cfg(all(target_arch = "x86_64", not(any(feature = "std", test))))]
core::arch::global_asm!(
    ".global strtold_l",
    ".type strtold_l, @function",
    "strtold_l:",
    "    jmp strtold",
    options(att_syntax),
);

/// Converts the initial portion of a string to a `long` using the C/POSIX locale.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtol_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
    _locale: *mut c_void,
) -> c_long {
    unsafe { strtol(nptr, endptr, base) }
}

/// Converts the initial portion of a string to a `long long` using the C/POSIX locale.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtoll_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
    _locale: *mut c_void,
) -> c_longlong {
    unsafe { strtoll(nptr, endptr, base) }
}

/// Converts the initial portion of a string to an `unsigned long` using the C/POSIX locale.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtoul_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
    _locale: *mut c_void,
) -> c_ulong {
    unsafe { strtoul(nptr, endptr, base) }
}

/// Converts the initial portion of a string to an `unsigned long long` using the C/POSIX locale.
///
/// # Safety
///
/// This function dereferences the raw pointers `nptr` and `endptr`, which must be valid.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtoull_l(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
    _locale: *mut c_void,
) -> c_ulonglong {
    unsafe { strtoull(nptr, endptr, base) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
