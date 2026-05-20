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

use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Benchmark entry point. Creates a VM snapshot and exits. When restored from
/// a snapshot the execution resumes here and the process exits immediately,
/// allowing the snapshot restore latency to be measured.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::sys::kcall::pm::snapshot()?;
    Ok(())
}
