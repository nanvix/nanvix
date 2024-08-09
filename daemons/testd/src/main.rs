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

use nvx::{
    mm::Address,
    sys::config,
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
                nvx::log!("{} {}", "passed", stringify!($fn_name)),
            false =>
                panic!("{} {}", "FAILED", stringify!($fn_name)),
        }
    }};
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() {
    // Wait unblock message from the process manager daemon.
    ::nvx::log!("waiting for unblock message from process manager daemon...");
    if let Err(e) = ::nvx::ipc::recv() {
        ::nvx::log!("failed to receive ack message (error={:?})", e);
    }

    ::nvx::log!("Running test server...");

    pm::test();
    event::test();
    mm::test();

    // Lookup memory daemon PID.
    let name: &str = "memd";
    ::nvx::log!("looking up memory daemon PID...");
    match ::nvx::pm::lookup(name) {
        Ok(pid) => {
            ::nvx::log!("memory daemon PID: {:?}", pid);
        },
        Err(e) => {
            ::nvx::log!("failed to lookup memory daemon PID (error={:?})", e);
        },
    }

    // Force a page fault.
    ::nvx::log!("triggering a page fault...");
    unsafe {
        let ptr: *mut u8 = config::memory_layout::USER_HEAP_BASE.into_raw_value() as *mut u8;
        *ptr = 1;
    }

    unreachable!("the test daemon should have been killed");
}
