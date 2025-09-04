// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This program prints numbers sequentially for a long time. It can be used for pausing and
//! resuming an application and checking that the count continues from where it left off, and also
//! for taking a snapshot of the VM state when paused, then checking that the count continues from
//! where it left off when loading the snapshot.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate nvx;

use alloc::{
    format,
    string::String,
};
use sys::{
    error::Error,
    kcall::debug,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    const ITERATIONS_STEP: u64 = 500_000_000;

    for i in 0..u64::MAX {
        if i % ITERATIONS_STEP == 0 {
            let string: String = format!("{}\n", i / ITERATIONS_STEP);
            let _ = debug::debug(string.as_ptr(), string.len());
        }
    }

    Ok(())
}
