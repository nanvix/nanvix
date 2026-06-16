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

use ::proc::ProcessDaemon;
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let mut procd: ProcessDaemon = match ProcessDaemon::init() {
        Ok(procd) => procd,
        Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
    };

    let exit_status: i32 = procd.run();
    procd.shutdown();

    // Propagate the exit status of the triggering process so that the kernel
    // reports it as the VM exit code (procd is the last process to exit).
    // NOTE: we panic on error because __kcall_exit never returns on success.
    let Err(error) = ::sys::kcall::pm::__kcall_exit(exit_status);
    panic!("failed to exit process (error={error:?})");
}
