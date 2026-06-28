// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![cfg_attr(not(feature = "std"), no_std)]
// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
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
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

pub mod btowc;
pub mod mbstate;
pub mod multibyte;
pub mod utf8;
pub mod wchar_t;
pub mod wcs_narrow;
pub mod wcscat;
pub mod wcschr;
pub mod wcscmp;
pub mod wcscoll;
pub mod wcscpy;
pub mod wcsftime;
pub mod wcslen;
pub mod wcsncmp;
pub mod wcsncpy;
pub mod wcsrchr;
pub mod wcsstr;
pub mod wcstod;
pub mod wcstok;
pub mod wcstol;
pub mod wcstoll;
pub mod wcstoul;
pub mod wcstoull;
pub mod wcsxfrm;
pub mod wctob;
pub mod wmemchr;
pub mod wmemcmp;
pub mod wmemcpy;
pub mod wmemmove;
pub mod wmemset;

//==================================================================================================
// Test Support
//==================================================================================================

// The conversion routines reference `__errno_location` to report `EILSEQ`. That symbol is provided
// by the C library in the guest build, but the MSVC C runtime used for host `std` unit tests on
// Windows does not export this glibc/musl symbol, so supply a thread-local definition here to let
// the errno-setting paths link and run. On Linux hosts the system C library already provides the
// symbol, so the shim is not compiled there.
#[cfg(all(test, feature = "std", target_os = "windows"))]
mod windows_errno_shim {
    use ::core::cell::Cell;
    use ::sysapi::ffi::c_int;

    std::thread_local! {
        static ERRNO: Cell<c_int> = const { Cell::new(0) };
    }

    /// Host-only definition of `__errno_location` for Windows `std` unit tests.
    ///
    /// # Safety
    ///
    /// The returned pointer is valid for the lifetime of the calling thread.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __errno_location() -> *mut c_int {
        ERRNO.with(Cell::as_ptr)
    }
}
