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

use ::sys::error::Error;
use ::syscall::unistd::do_execv;

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the target program in the mounted ramfs (mounted at the filesystem root).
const TARGET_PATH: &str = "/target";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point of the `execv()` test. It replaces its own image with the target program loaded
/// from the mounted ramfs. On success this does not return: the target program runs in place and
/// writes the success sentinel. Reaching the code after [`do_execv`] therefore indicates failure.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Replace this image with the target program. The argument vector's first element is the
    // conventional program name.
    let error: Error = do_execv(TARGET_PATH, &["target"], &[]);

    // Only reached if execv() failed; surface the error so the test fails with a non-zero status.
    ::syslog::error!("execv({TARGET_PATH}) failed: {error:?}");
    Err(error)
}
