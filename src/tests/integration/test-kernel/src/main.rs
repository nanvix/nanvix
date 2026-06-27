// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Modules
//==================================================================================================

mod demand_paging;
mod detach;
mod direction_flag;
mod duplicate;
mod getppid;
mod mmio_ramfs;
mod rendezvous;
mod source_spoofing;
mod tls;

//==================================================================================================
// Constants
//==================================================================================================

/// Exit code to return from this test program.
const EXIT_CODE: i32 = 13;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point of the test program. This function runs kernel interface tests and returns an
/// error with a specific error code to test that the expected_exit_code assertion works correctly
/// in nanvix-test.
///
/// # Returns
///
/// Returns an error with the specified exit code.
///
#[no_mangle]
pub fn main() -> Result<(), Error> {
    detach::run()?;

    duplicate::run()?;

    getppid::run()?;

    mmio_ramfs::run()?;

    tls::run()?;

    direction_flag::run()?;

    demand_paging::run()?;

    rendezvous::run()?;

    source_spoofing::run()?;

    // Return an error with the specified exit code.
    // The nvx runtime will convert this to the process exit code.
    Err(Error::new(
        ErrorCode::try_from(EXIT_CODE as i64).unwrap_or(ErrorCode::OperationNotPermitted),
        "intentional exit with non-zero code for testing",
    ))
}
