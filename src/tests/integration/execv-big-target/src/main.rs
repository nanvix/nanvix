// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the inflation blob: one eighth of the guest's physical memory.
///
/// This makes the program's on-disk image large (`MEMORY_SIZE / 8`), so that loading it through
/// `execv()` exercises the large-binary path. The value is derived from the build-time guest
/// memory size, so it scales with the configured `MEMORY_SIZE`.
const BIG_SIZE: usize = ::config::kernel::MEMORY_SIZE / 8;

/// Sentinel byte stored throughout the blob and checked at runtime to confirm the segment loaded.
const FILL: u8 = 0xAB;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Large, non-zero-initialized blob that inflates the program's on-disk image and its loadable
/// data segment to [`BIG_SIZE`] bytes. Being non-zero forces it into a file-backed (PROGBITS)
/// section rather than BSS, so the on-disk image actually grows. `#[used]` together with the
/// runtime reads in [`main`] prevents it from being optimized or garbage-collected away.
#[used]
static BIG_BLOB: [u8; BIG_SIZE] = [FILL; BIG_SIZE];

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point of the large target program loaded by the `execv()` big-binary test. It validates
/// that the first and last bytes of its large data segment were loaded correctly (confirming the
/// whole image was read in), writes the success sentinel, and exits.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Read the first and last bytes of the large segment. `black_box` keeps these reads (and hence
    // the blob) from being optimized away, and confirms the segment was fully loaded by execv().
    let first: u8 = ::core::hint::black_box(BIG_BLOB[0]);
    let last: u8 = ::core::hint::black_box(BIG_BLOB[BIG_SIZE - 1]);

    if first == FILL && last == FILL {
        unistd::write(STDOUT_FILENO, "ok".as_bytes())?;
        Ok(())
    } else {
        unistd::write(STDOUT_FILENO, "failed".as_bytes())?;
        Err(Error::new(ErrorCode::InvalidArgument, "large data segment was not loaded correctly"))
    }
}
