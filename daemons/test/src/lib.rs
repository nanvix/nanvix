// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

/// Tests process management kernel calls.
mod pm;

/// Tests memory management kernel calls.
mod mm;

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
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
    ::nvx::log!("Running test server...");
    pm::test();
    mm::test();

    // Send unblock message to the init daemon.
    let message: Message = Message::new(
        ProcessIdentifier::from(1),
        ProcessIdentifier::from(2),
        [0; Message::SIZE],
        MessageType::Ipc,
    );
    if let Err(e) = ::nvx::ipc::send(&message) {
        ::nvx::log!("failed to unblock init (error={:?})", e);
    }

    // Wait ack message from the init daemon.
    if let Err(e) = ::nvx::ipc::recv() {
        ::nvx::log!("failed to receive ack message (error={:?})", e);
    }

    // Force a page fault.
    unsafe {
        let ptr: *mut u8 = 0xa0000000 as *mut u8;
        *ptr = 1;
    }

    unreachable!("the test daemon should have been killed by init");
}
