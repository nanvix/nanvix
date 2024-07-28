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
    ipc::{
        Message,
        MessageType,
    },
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
    // Wait unblock message from the test daemon.
    if let Err(e) = ::nvx::ipc::recv() {
        panic!("failed to receive unblock message (error={:?})", e);
    }

    ::nvx::log!("running init daemon...");

    // Acquire exception management capability.
    ::nvx::log!("acquiring exception management capability...");
    if let Err(e) = ::nvx::pm::capctl(Capability::ExceptionControl, true) {
        panic!("failed to acquire exception management capability (error={:?})", e);
    }

    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Register exception handler for page faults.
    ::nvx::log!("subscribing to page faults...");
    if let Err(e) =
        ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Register)
    {
        panic!("failed to subscribe to page faults (error={:?})", e);
    }

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

    let info: EventInformation = match ::nvx::ipc::recv() {
        Ok(message) => EventInformation::from(message),
        Err(e) => panic!("failed to receive exception message (error={:?})", e),
    };

    // Terminate test daemon.
    ::nvx::log!("terminating test daemon...");
    if let Err(e) = ::nvx::pm::terminate(info.pid) {
        panic!("failed to terminate test daemon (error={:?})", e);
    }

    // Acknowledge exception event.
    if let Err(e) = ::nvx::event::resume(info.id) {
        panic!("failed to resume exception event (error={:?})", e);
    }

    let magic_string: &str = "PANIC: Hello World!\n";
    let _ = ::nvx::debug::debug(magic_string.as_ptr(), magic_string.len());
    loop {
        core::hint::spin_loop()
    }
}
