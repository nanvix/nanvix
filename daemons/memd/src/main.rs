// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

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
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        Capability,
        ProcessIdentifier,
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
        ShutdownMessage,
    },
    sys::error::Error,
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

fn handle_ipc_request(message: Message) -> Result<bool, Error> {
    let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

    // Parse message.
    match message.header {
        // Parse process management message.
        SystemMessageHeader::ProcessManagement => {
            let message: ProcessManagementMessage =
                ProcessManagementMessage::from_bytes(message.payload)?;
            // Parse operation.
            match message.header {
                ProcessManagementMessageHeader::Shutdown => {
                    let message: ShutdownMessage = ShutdownMessage::from_bytes(message.payload);
                    ::nvx::log!("shutting down (code={:?})...", message.code);
                    return Ok(true);
                },
                _ => {
                    ::nvx::log!("received unknown process management message, ignoring...");
                },
            }
        },
        // Parse memory management message.
        SystemMessageHeader::MemoryManagement => {
            ::nvx::log!("received memory management message, ignoring...");
        },
        // Parse filesystem management message.
        SystemMessageHeader::FilesystemManagement => {
            ::nvx::log!("received filesystem management message, ignoring...");
        },
    }

    Ok(false)
}

#[no_mangle]
pub fn main() {
    let mypid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => panic!("failed to get pid (error={:?})", e),
    };
    let myname: &str = "memd";

    ::nvx::log!("running memory management daemon (pid={:?})...", mypid);

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

    // Signup to the process manager daemon.
    if let Err(e) = ::nvx::pm::signup(&mypid, &myname) {
        panic!("failed to signup to process manager daemon (error={:?})", e);
    }

    loop {
        match ::nvx::ipc::recv() {
            Ok(message) => match message.message_type {
                MessageType::Exception => handle_page_fault(EventInformation::from(message)),
                MessageType::Ipc => match handle_ipc_request(message) {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(e) => ::nvx::log!("failed to handle ipc request (error={:?})", e),
                },
                MessageType::Empty => unreachable!("should not receive empty messages"),
                MessageType::Interrupt => unreachable!("should not receive interrupts"),
                MessageType::Ikc => unreachable!("should not receive ikc messages"),
                MessageType::SchedulingEvent => {
                    unreachable!("should not receive scheduling events")
                },
            },
            Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
        }
    }

    // Shutdown memory management daemon.
    if let Err(e) = ::nvx::pm::exit(0) {
        ::nvx::log!("failed to shutdown memory management daemon (error={:?})", e);
    }

    loop {
        ::core::hint::spin_loop();
    }
}
