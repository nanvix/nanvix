// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
// Use no_std except during tests so the Rust test harness (which requires std) can run.
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_wrap)]
// `deny` (not `forbid`) so the inherently-lossy integer→float conversions in the
// float parsers (`strtod`/`strtof`) can opt out with a justified, local
// `#[allow(clippy::cast_precision_loss)]`. `forbid` would make those `#[allow]`s
// a hard E0453 error; the conversions are unavoidable when building an `f64`/`f32`
// from parsed digits.
#![deny(clippy::cast_precision_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
// `clippy::cast_possible_truncation` is handled per target pointer width. On 32-bit targets `usize`
// and `c_size_t` are both 32-bit, so width-narrowing casts are harmless and the lint is allowed. On
// 64-bit targets it is `deny` (not `forbid`) so that the unavoidable `f64 as f32` narrowing in
// `strtof` can opt out with a justified, local `#[allow(clippy::cast_possible_truncation)]`,
// mirroring the `cast_precision_loss` handling above; `forbid` would make that `#[allow]` a hard
// E0453.
#![cfg_attr(target_pointer_width = "32", allow(clippy::cast_possible_truncation))]
#![cfg_attr(
    not(target_pointer_width = "32"),
    deny(clippy::cast_possible_truncation)
)]

//==================================================================================================
// Macros
//==================================================================================================

/// Crate-local diagnostic logging shim.
///
/// On the guest (`no_std`) build this forwards to `syslog`'s `warn!` macro. Under the `std` feature
/// used by host unit tests, `syslog` exposes no logging macros (its standalone logger relies on
/// guest-only kernel calls), so the shim expands to a no-op that still references its arguments to
/// avoid unused-variable warnings.
///
/// The shim is made available through textual scoping (it is defined before the module declarations
/// below) rather than a `pub(crate) use`, because the name `warn` collides with the built-in `warn`
/// attribute when imported through a path.
#[cfg(not(feature = "std"))]
macro_rules! warn {
    ($($arg:tt)*) => { ::syslog::warn!($($arg)*) };
}

#[cfg(feature = "std")]
macro_rules! warn {
    ($($arg:tt)*) => {{ let _ = ::core::format_args!($($arg)*); }};
}

//==================================================================================================
// Modules
//==================================================================================================

mod abort;
mod abs_fn;
mod aligned_alloc;
mod atexit;
mod atof;
mod atoi;
mod atol;
mod atoll;
mod block_header;
mod bsearch;
mod calloc;
mod div_fn;
pub mod env_table;
mod exit;
mod free;
mod getenv;
mod labs;
mod ldiv;
mod llabs;
mod lldiv;
mod malloc;
mod malloc_usable_size;
mod memalign;
mod mkdtemp;
mod mkostemp;
mod mkstemp;
mod posix_memalign;
mod putenv;
mod qsort;
mod quick_exit;
mod rand;
mod realloc;
mod reallocarray;
mod secure_getenv;
mod setenv;
mod strtod;
mod strtof;
mod strtol;
mod strtold;
mod strtoll;
mod strtoul;
mod strtoull;
mod system;
mod tmpname;
mod unsetenv;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use abort::abort;
pub use abs_fn::abs;
pub use aligned_alloc::aligned_alloc;
pub use atexit::atexit;
pub use atof::atof;
pub use atoi::atoi;
pub use atol::atol;
pub use atoll::atoll;
pub use bsearch::bsearch;
pub use calloc::calloc;
pub use div_fn::{
    div,
    div_t,
};
pub use exit::{
    _Exit,
    exit,
};
pub use free::free;
pub use getenv::getenv;
pub use labs::labs;
pub use ldiv::{
    ldiv,
    ldiv_t,
};
pub use llabs::llabs;
pub use lldiv::{
    lldiv,
    lldiv_t,
};
pub use malloc::malloc;
pub use malloc_usable_size::malloc_usable_size;
pub use memalign::memalign;
pub use mkdtemp::mkdtemp;
pub use mkostemp::mkostemp;
pub use mkstemp::mkstemp;
pub use posix_memalign::posix_memalign;
pub use putenv::putenv;
pub use qsort::{
    qsort,
    qsort_r,
};
pub use quick_exit::{
    at_quick_exit,
    quick_exit,
};
pub use rand::{
    rand,
    srand,
};
pub use realloc::realloc;
pub use reallocarray::reallocarray;
pub use secure_getenv::secure_getenv;
pub use setenv::setenv;
pub use strtod::strtod;
pub use strtof::strtof;
pub use strtol::strtol;
pub use strtold::strtold;
pub use strtoll::strtoll;
pub use strtoul::strtoul;
pub use strtoull::strtoull;
pub use system::system;
pub use unsetenv::unsetenv;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes `code` to `errno`.
///
/// # Parameters
///
/// - `code`: Error code to be written to `errno`.
///
#[inline(always)]
fn set_errno(code: c_int) {
    // SAFETY: `__errno_location()` returns a valid pointer to `errno`.
    unsafe {
        *__errno_location() = code;
    }
}

//==================================================================================================
// Test Support
//==================================================================================================

/// Host-only `__errno_location` shim for unit tests.
///
/// On the guest, `errno` is provided by the C runtime. The Windows host test binary has no such
/// symbol (the MSVC runtime exposes `_errno` instead), so this supplies a thread-local stand-in.
/// Linux hosts resolve the symbol from their C library (glibc/musl) and therefore must not redefine
/// it. The shim lives in its own module so that its `#[no_mangle]` definition provides the global C
/// symbol without colliding with the `sysapi::errno::__errno_location` import used above.
#[cfg(all(test, feature = "std", not(target_os = "linux")))]
mod errno_shim {
    #[unsafe(no_mangle)]
    extern "C" fn __errno_location() -> *mut ::sysapi::ffi::c_int {
        thread_local! {
            static ERRNO: ::core::cell::UnsafeCell<::sysapi::ffi::c_int> =
                const { ::core::cell::UnsafeCell::new(0) };
        }
        ERRNO.with(|cell| cell.get())
    }
}
