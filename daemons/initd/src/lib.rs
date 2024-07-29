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
    // Wait unblock message from the test daemon.
    if let Err(e) = ::nvx::ipc::recv() {
        panic!("failed to receive unblock message (error={:?})", e);
    }

    ::nvx::log!("running init daemon...");

    // Unblock memory daemon.
    ::nvx::log!("unblocking memory daemon...");
    let message: Message = Message::new(
        ProcessIdentifier::from(2),
        ProcessIdentifier::from(3),
        [0; Message::SIZE],
        MessageType::Ipc,
    );
    if let Err(e) = ::nvx::ipc::send(&message) {
        panic!("failed to unblock memory daemon (error={:?})", e);
    }

    // Wait for ack message from memory daemon.
    ::nvx::log!("waiting for ack message from memory daemon...");
    let _message: Message = match ::nvx::ipc::recv() {
        Ok(message) => message,
        Err(e) => panic!("failed to receive ack message from memory daemon (error={:?})", e),
    };

    // Ack test daemon.
    ::nvx::log!("sending ack message to test daemon...");
    let message: Message = Message::new(
        ProcessIdentifier::from(2),
        ProcessIdentifier::from(1),
        [0; Message::SIZE],
        MessageType::Ipc,
    );
    if let Err(e) = ::nvx::ipc::send(&message) {
        panic!("failed to unblock test daemon(error={:?})", e);
    }

    loop {
        core::hint::spin_loop()
    }
}
