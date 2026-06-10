// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::proc::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
};
use ::sys::{
    error::Error,
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
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn handle_page_fault(info: EventInformation) {
    // Terminate process.
    ::syslog::info!("terminating process (pid={:?})", info.pid);
    if let Err(e) = ::sys::kcall::pm::__kcall_terminate(info.pid) {
        panic!("failed to terminate test daemon (error={:?})", e);
    }

    // Acknowledge exception event.
    if let Err(e) = ::sys::kcall::event::__kcall_resume(info.id) {
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
                    ::syslog::info!("shutting down (code={:?})...", message.code);
                    return Ok(true);
                },
                _ => {
                    ::syslog::warn!("received unknown process management message, ignoring...");
                },
            }
        },
        // Parse memory management message.
        SystemMessageHeader::MemoryManagement => {
            ::syslog::warn!("received memory management message, ignoring...");
        },
        // Parse filesystem management message.
        SystemMessageHeader::FilesystemManagement => {
            ::syslog::warn!("received filesystem management message, ignoring...");
        },
    }

    Ok(false)
}

#[unsafe(no_mangle)]
pub fn main() {
    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(e) => panic!("failed to get pid (error={:?})", e),
    };
    let myname: &str = ::config::daemons::MEMD_NAME;

    ::syslog::info!("running memory management daemon (pid={:?})...", mypid);

    // Acquire exception management capability.
    ::syslog::info!("acquiring exception management capability...");
    if let Err(e) = ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        panic!("failed to acquire exception management capability (error={:?})", e);
    }

    // Acquire process management capability so that faulting processes can be terminated.
    ::syslog::info!("acquiring process management capability...");
    if let Err(e) = ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        panic!("failed to acquire process management capability (error={:?})", e);
    }

    // Signup to the process manager daemon.
    // NOTE: this must happen before subscribing to page faults so that no
    // exception messages arrive during the synchronous signup handshake.
    if let Err(e) = ::proc::signup(&mypid, myname) {
        panic!("failed to signup to process manager daemon (error={:?})", e);
    }

    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Subscribe to page faults.
    ::syslog::info!("subscribing to page faults...");
    if let Err(e) = ::sys::kcall::event::__kcall_evctrl(
        Event::Exception(page_fault_exception),
        EventCtrlRequest::Register,
    ) {
        panic!("failed to subscribe to page faults (error={:?})", e);
    }

    loop {
        match ::sys::kcall::ipc::__kcall_recv() {
            Ok(message) => match message.message_type {
                MessageType::Exception => match EventInformation::try_from(message) {
                    Ok(info) => handle_page_fault(info),
                    Err(e) => {
                        ::syslog::error!("failed to parse event information (error={:?})", e)
                    },
                },
                MessageType::Ipc => match handle_ipc_request(message) {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(e) => ::syslog::error!("failed to handle ipc request (error={:?})", e),
                },
                MessageType::Interrupt => unreachable!("should not receive interrupts"),
                MessageType::Ikc => unreachable!("should not receive ikc messages"),
                MessageType::ProcessTerminationEvent => {
                    unreachable!("should not receive process termination events")
                },
                MessageType::ProcessCreationEvent => {
                    unreachable!("should not receive process creation events")
                },
                MessageType::PullResponse => {
                    ::syslog::error!("received unexpected pull response, ignoring");
                    continue;
                },
            },
            Err(e) => ::syslog::error!("failed to receive exception message (error={:?})", e),
        }
    }

    // Shutdown memory management daemon.
    let e = ::sys::kcall::pm::__kcall_exit(0);
    ::syslog::error!("failed to shutdown memory management daemon (error={:?})", e);

    loop {
        ::core::hint::spin_loop();
    }
}
