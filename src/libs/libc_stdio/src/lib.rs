// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
// Use no_std except during tests so the Rust test harness (which requires std) can run.
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
// Features
#![feature(c_variadic)]
// Lints
#![forbid(clippy::unwrap_used)]
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
// The following lints need to be handled case-by-case depending on the target pointer width.
// C-interop casts between Rust native types and fixed-width C ABI types are inherent to printf
// formatting. On the 32-bit Nanvix target, these casts are no-ops.
#![cfg_attr(target_pointer_width = "32", expect(clippy::cast_possible_truncation))]
#![cfg_attr(
    not(target_pointer_width = "32"),
    expect(clippy::cast_possible_truncation)
)]
#![cfg_attr(target_pointer_width = "32", expect(clippy::cast_possible_wrap))]
#![cfg_attr(not(target_pointer_width = "32"), expect(clippy::cast_possible_wrap))]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

mod float_fmt;
mod format_engine;
mod streams;

pub mod asprintf;
pub mod clearerr;
pub mod dprintf;
pub mod fclose;
pub mod fdopen;
pub mod feof;
pub mod ferror;
pub mod fflush;
pub mod fgetc;
pub mod fgets;
pub mod fileno;
pub mod flockfile;
pub mod fopen;
pub mod fprintf;
pub mod fputc;
pub mod fputs;
pub mod fread;
pub mod freopen;
pub mod fseek;
pub mod fseeko;
pub mod ftell;
pub mod ftello;
pub mod fwrite;
pub mod getchar;
pub mod getdelim;
pub mod pclose;
pub mod perror;
pub mod popen;
pub mod printf;
pub mod putchar;
pub mod puts;
pub mod remove;
pub mod rename;
pub mod rewind;
pub mod setbuf;
pub mod snprintf;
pub mod sprintf;
pub mod sscanf;
pub mod swprintf;
pub mod tmpfile;
pub mod ungetc;
pub mod vasprintf;
pub mod vdprintf;
pub mod vfprintf;
pub mod vprintf;
pub mod vsnprintf;
pub mod vsprintf;
pub mod vsscanf;

//==================================================================================================
// Exports
//==================================================================================================

pub use streams::{
    stderr,
    stdin,
    stdout,
    FILE,
};

//==================================================================================================
// Test Support
//==================================================================================================

// The MSVC C runtime used for Windows host unit tests does not export the glibc/musl
// `__errno_location` symbol used by the guest libc build, so provide a thread-local test shim.
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
