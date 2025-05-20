// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

/// Tests event management kernel calls.
mod event;

/// Tests process management kernel calls.
mod pm;

/// Tests memory management kernel calls.
mod mm;

//==================================================================================================
// Imports
//==================================================================================================

extern crate nvx;

use ::sys::{
    config,
    mm::Address,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Macros
//==================================================================================================

///
/// **Description**
///
/// Runs test and prints whether it passed or failed on the standard output.
///
#[macro_export]
macro_rules! test {
    ($fn_name:ident($($arg:expr),*)) => {{
        match $fn_name($($arg),*) {
            true =>
                ::syslog::info!("{} {}", "passed", stringify!($fn_name)),
            false =>
                panic!("{} {}", "FAILED", stringify!($fn_name)),
        }
    }};
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() {
    ::syslog::info!("Running test server...");

    pm::test();
    event::test();
    mm::test();

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => panic!("failed to get process identifier (error={:?})", e),
    };

    let myname: &str = "testd";

    // Signup to the process manager daemon.
    if let Err(e) = ::proc::signup(&mypid, myname) {
        panic!("failed to signup to process manager daemon (error={:?})", e);
    }

    // Make sure that memory daemon is running.
    loop {
        match ::proc::lookup("memd") {
            Ok(_) => {
                ::syslog::info!("memory daemon is running");
                break;
            },
            Err(e) => {
                ::syslog::error!("memory daemon is not running (error={:?})", e);
            },
        }
    }

    // Force a page fault.
    ::syslog::info!("triggering a page fault...");
    unsafe {
        let ptr: *mut u8 = config::memory_layout::USER_HEAP_BASE.into_raw_value() as *mut u8;
        *ptr = 1;
    }

    unreachable!("the test daemon should have been killed");
}
