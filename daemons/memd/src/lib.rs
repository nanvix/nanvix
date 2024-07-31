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

fn handle_page_fault(info: EventInformation) {
    // Terminate process.
    ::nvx::log!("terminating process (pid={:?})", info.pid);
    if let Err(e) = ::nvx::pm::terminate(info.pid) {
        panic!("failed to terminate test daemon (error={:?})", e);
    }

    // Acknowledge exception event.
    if let Err(e) = ::nvx::event::resume(info.id) {
        panic!("failed to resume exception event (error={:?})", e);
    }
}

fn handle_ipc_requests() {
    unimplemented!()
}

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

    // Acquire exception management capability.
    ::nvx::log!("acquiring exception management capability...");
    if let Err(e) = ::nvx::pm::capctl(Capability::ExceptionControl, true) {
        panic!("failed to acquire exception management capability (error={:?})", e);
    }

    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Subscribe to page faults.
    ::nvx::log!("subscribing to page faults...");
    if let Err(e) =
        ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Register)
    {
        panic!("failed to subscribe to page faults (error={:?})", e);
    }

    // Ack init daemon.
    ::nvx::log!("sending ack message to init daemon...");
    let message: Message =
        Message::new(mypid, ProcessIdentifier::from(2), [0; Message::SIZE], MessageType::Ipc);
    if let Err(e) = ::nvx::ipc::send(&message) {
        panic!("failed to ack init (error={:?})", e);
    }

    loop {
        match ::nvx::ipc::recv() {
            Ok(message) => match message.message_type {
                MessageType::Exception => handle_page_fault(EventInformation::from(message)),
                MessageType::Ipc => handle_ipc_requests(),
                MessageType::Interrupt => unreachable!("should not receive interrupts"),
                MessageType::SchedulingEvent => {
                    unreachable!("should not receive scheduling events")
                },
            },
            Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
        }
    }
}
