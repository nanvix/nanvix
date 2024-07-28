// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]

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
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() {
    let mypid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => panic!("failed to get pid (error={:?})", e),
    };

    ::nvx::log!("running memory management daemon (pid={:?})...", mypid);

    // Wait unblock message from the init daemon.
    if let Err(e) = ::nvx::ipc::recv() {
        panic!("failed to receive unblock message (error={:?})", e);
    }

    // Ack init daemon.
    ::nvx::log!("sending ack message to init daemon...");
    let message: Message =
        Message::new(mypid, ProcessIdentifier::from(2), [0; Message::SIZE], MessageType::Ipc);
    if let Err(e) = ::nvx::ipc::send(&message) {
        panic!("failed to ack init (error={:?})", e);
    }

    loop {
        ::core::hint::spin_loop();
    }
}
