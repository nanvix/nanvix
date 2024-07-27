// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    event::{
        Event,
        EventCtrlRequest,
        EventInformation,
        ExceptionEvent,
    },
    ipc::Message,
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() {
    if let Err(e) = ::nvx::ipc::recv() {
        ::nvx::log!("failed to receive unblock message (error={:?})", e);
    }

    ::nvx::log!("Running init server...");
    let magic_string: &str = "PANIC: Hello World!\n";
    let _ = ::nvx::debug::debug(magic_string.as_ptr(), magic_string.len());
    loop {
        core::hint::spin_loop()
    }
}
