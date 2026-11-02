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
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
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

pub mod imaxabs;
pub mod imaxdiv;
pub mod inttypes;
pub mod strtoimax;
pub mod strtoumax;

//==================================================================================================
// Test Support
//==================================================================================================

// The conversion routines reference `__errno_location` to report `ERANGE`. That symbol is provided
// by the C library in the guest build, but the MSVC C runtime used for host `std` unit tests on
// Windows does not export this glibc/musl symbol, so supply a thread-local definition here to let
// the `errno`-asserting tests link and run. On Linux hosts the system C library already provides
// the symbol, so the shim is not compiled there.
#[cfg(all(test, feature = "std", target_os = "windows"))]
#[path = "tests/windows_errno_shim.rs"]
mod windows_errno_shim;
