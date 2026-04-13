// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]
#![cfg_attr(feature = "hyperlight", allow(dead_code))]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;

use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Modules
//==================================================================================================

mod demand_paging;
mod direction_flag;
#[cfg(not(feature = "hyperlight"))]
mod mmio_ramfs;
#[cfg(not(feature = "hyperlight"))]
mod rendezvous;
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
    #[cfg(not(feature = "hyperlight"))]
    mmio_ramfs::run()?;

    // TLS tests cause GS:0 null deref from Rust's TLS infrastructure.
    // The kernel's set_thread_data_area updates GDT+GS but the compiler
    // emits %gs:0x0 reads that happen before the TDA is set up.
    #[cfg(not(feature = "hyperlight"))]
    tls::run()?;

    direction_flag::run()?;

    #[cfg(not(feature = "hyperlight"))]
    demand_paging::run()?;

    #[cfg(not(feature = "hyperlight"))]
    rendezvous::run()?;

    // Return an error with the specified exit code.
    // The nvx runtime will convert this to the process exit code.
    Err(Error::new(
        ErrorCode::try_from(EXIT_CODE as i64).unwrap_or(ErrorCode::OperationNotPermitted),
        "intentional exit with non-zero code for testing",
    ))
}
