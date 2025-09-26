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

//==================================================================================================
// Modules
//==================================================================================================

mod aligned_alloc;
mod block_header;
mod calloc;
mod free;
mod malloc;
mod posix_memalign;
mod realloc;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::sysapi::{
    errno::__errno_location,
    ffi::c_int,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use aligned_alloc::aligned_alloc;
pub use calloc::calloc;
pub use free::free;
pub use malloc::malloc;
pub use posix_memalign::posix_memalign;
pub use realloc::realloc;

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
