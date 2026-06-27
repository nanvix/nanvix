// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

/// Must come first.
extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

mod tests;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Exit code returned by the parent process once every check passes.
///
/// Success is signaled through a distinctive non-zero exit code rather than a stdout marker, so the
/// test harness can assert on an exact value. A crash or an unexpected early exit surfaces a
/// different code and is therefore reported as a failure instead of a false success. This mirrors
/// the convention used by `test-fork-guestfs`.
const EXIT_CODE: i32 = 13;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    tests::run_all()?;

    // Only the parent process reaches this point; the child terminates via `exit()` inside the
    // test. The nvx runtime converts this error into the process exit code consumed by the harness.
    Err(Error::new(
        ErrorCode::try_from(EXIT_CODE).unwrap_or(ErrorCode::OperationNotPermitted),
        "fork() host-filesystem descriptor duplication suite completed successfully",
    ))
}
