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
        SchedulingEvent,
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

    let mypid: ProcessIdentifier = ::nvx::pm::getpid().expect("failed to get pid");

    ::nvx::log!("running init daemon...");

    // Acquire process management capabilities.
    ::nvx::log!("acquiring process managemnet capabilities...");
    if let Err(e) = ::nvx::pm::capctl(Capability::ProcessManagement, true) {
        panic!("failed to acquire process management capabilities (error={:?})", e);
    }

    // Subscribe to process termination.
    ::nvx::log!("subscribing to process termination...");
    if let Err(e) = ::nvx::event::evctrl(
        Event::Scheduling(SchedulingEvent::ProcessTermination),
        EventCtrlRequest::Register,
    ) {
        panic!("failed to subscribe to process termination (error={:?})", e);
    }

    // Unblock memory daemon.
    ::nvx::log!("unblocking memory daemon...");
    let message: Message =
        Message::new(mypid, ProcessIdentifier::from(3), [0; Message::SIZE], MessageType::Ipc);
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
    let message: Message =
        Message::new(mypid, ProcessIdentifier::from(1), [0; Message::SIZE], MessageType::Ipc);
    if let Err(e) = ::nvx::ipc::send(&message) {
        panic!("failed to unblock test daemon(error={:?})", e);
    }

    loop {
        match ::nvx::ipc::recv() {
            Ok(message) => match message.message_type {
                MessageType::Exception => unreachable!("should not receive exceptions"),
                MessageType::Ipc => unreachable!("should not receive IPC messages"),
                MessageType::Interrupt => unreachable!("should not receive interrupts"),
                MessageType::SchedulingEvent => {
                    // Deserialize process identifier.
                    let pid: ProcessIdentifier = ProcessIdentifier::from(u32::from_le_bytes(
                        message.payload[0..4].try_into().unwrap(),
                    )
                        as usize);

                    // Deserialize process status.
                    let status: i32 = i32::from_le_bytes(message.payload[4..8].try_into().unwrap());
                    ::nvx::log!("process terminated (pid={:?}, status={:?})", pid, status);

                    if pid == ProcessIdentifier::from(1) {
                        let magic_string: &str = "PANIC: Hello World!\n";
                        let _ = ::nvx::debug::debug(magic_string.as_ptr(), magic_string.len());
                    }
                },
            },
            Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
        }
        core::hint::spin_loop()
    }
}
