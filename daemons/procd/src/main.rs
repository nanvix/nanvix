// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::alloc::{
    collections::btree_map::BTreeMap,
    string::{
        String,
        ToString,
    },
};
use ::core::{
    ffi::CStr,
    str,
};
use ::error::{
    Error,
    ErrorCode,
};
use ::nvx::{
    event::{
        Event,
        EventCtrlRequest,
        SchedulingEvent,
    },
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        Capability,
        LookupMessage,
        ProcessIdentifier,
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
        SignupMessage,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

struct ProcessDaemon {
    processes: BTreeMap<ProcessIdentifier, String>,
}

impl ProcessDaemon {
    // TODO: Change this, once we rename testd to initd.
    const INITD_NAME: &'static str = "testd";

    /// Initializes the process manager daemon.
    fn init() -> Result<Self, Error> {
        ::nvx::log!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::nvx::pm::getpid()?;
        assert_eq!(mypid, ProcessIdentifier::PROCD, "process daemon has unexpected pid");

        // Acquire process management capabilities.
        ::nvx::log!("acquiring process managemnet capabilities...");
        ::nvx::pm::capctl(Capability::ProcessManagement, true)?;

        // Subscribe to process termination.
        ::nvx::log!("subscribing to process termination...");
        ::nvx::event::evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Register,
        )?;

        Ok(Self {
            processes: BTreeMap::new(),
        })
    }

    /// Runs the process manager daemon.
    fn run(&mut self) {
        loop {
            match ::nvx::ipc::recv() {
                Ok(message) => {
                    ::nvx::log!("received message from={:?}", message.source,);
                    match message.message_type {
                        MessageType::Exception => unreachable!("should not receive exceptions"),
                        MessageType::Ipc => {
                            if let Err(e) = self.handle_ipc_message(message) {
                                ::nvx::log!("failed to handle IPC message (error={:?})", e);
                            }
                        },
                        MessageType::Empty => unreachable!("should not receive empty messages"),
                        MessageType::Interrupt => unreachable!("should not receive interrupts"),
                        MessageType::Ikc => unreachable!("should not receive IKC messages"),
                        MessageType::SchedulingEvent => {
                            match self.handle_scheduling_event(message) {
                                Ok(true) => break,
                                Ok(false) => continue,
                                Err(e) => {
                                    ::nvx::log!("failed to handle scheduling event (error={:?})", e)
                                },
                            }
                        },
                    }
                },
                Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
            }
        }
    }

    fn handle_scheduling_event(&mut self, message: Message) -> Result<bool, Error> {
        // Deserialize process identifier.
        let pid: ProcessIdentifier =
            ProcessIdentifier::from(u32::from_le_bytes(message.payload[0..4].try_into().unwrap()));

        ::nvx::log!("received scheduling event (pid={:?})", pid);

        // Deserialize process status.
        let status: i32 = i32::from_le_bytes(message.payload[4..8].try_into().unwrap());
        ::nvx::log!("process terminated (pid={:?}, status={:?})", pid, status);

        // De-register process.
        if let Some(name) = self.processes.remove(&pid) {
            ::nvx::log!("deregistering process (pid={:?}, name={:?})", pid, name);

            if name == Self::INITD_NAME {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn handle_ipc_message(&mut self, message: Message) -> Result<(), Error> {
        let destionation: ProcessIdentifier = message.source;
        let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

        ::nvx::log!("received system message (header={:?})", message.header);

        // Parse message.
        match message.header {
            // Parse process management message.
            SystemMessageHeader::ProcessManagement => {
                let message: ProcessManagementMessage =
                    ProcessManagementMessage::from_bytes(message.payload)?;

                // Parse operation.
                match message.header {
                    ProcessManagementMessageHeader::Signup => {
                        let message: SignupMessage = SignupMessage::from_bytes(message.payload);
                        self.handle_signup(destionation, message)?;
                    },
                    ProcessManagementMessageHeader::Lookup => {
                        let message: LookupMessage = LookupMessage::from_bytes(message.payload);
                        self.handle_lookup(destionation, message)?;
                    },
                    // Ignore all other messages.
                    _ => {},
                }
            },
            // Ignore all other messages.
            _ => {},
        }

        Ok(())
    }

    // Handles a signup message.
    fn handle_signup(
        &mut self,
        destination: ProcessIdentifier,
        message: SignupMessage,
    ) -> Result<(), Error> {
        let pid: ProcessIdentifier = message.pid;
        match CStr::from_bytes_until_nul(&message.name) {
            Ok(cstr) => match cstr.to_str() {
                Ok(name) => {
                    let s: String = name.to_string();

                    if s == "memd" {
                        ::nvx::log!("signup memory daemon");
                    } else {
                        ::nvx::log!("signup other process = {:?}", name);
                    }

                    ::nvx::log!("signing up process (pid={:?}, name={:?})", pid, s.as_bytes());
                    self.processes.insert(pid, s);
                    ::nvx::pm::signup_response(destination, pid, 0)?;
                },
                Err(_) => {
                    ::nvx::pm::signup_response(
                        destination,
                        pid,
                        ErrorCode::InvalidArgument.into_errno(),
                    )?;
                },
            },
            Err(_) => ::nvx::pm::signup_response(
                destination,
                pid,
                ErrorCode::InvalidArgument.into_errno(),
            )?,
        }

        Ok(())
    }

    // Handles a lookup message.
    pub fn handle_lookup(
        &self,
        destination: ProcessIdentifier,
        message: LookupMessage,
    ) -> Result<(), Error> {
        let name: &str = match CStr::from_bytes_until_nul(&message.name) {
            Ok(name) => match name.to_str() {
                Ok(s) => s,
                Err(_) => {
                    ::nvx::pm::lookup_response(
                        destination,
                        ProcessIdentifier::from(u32::MAX),
                        ErrorCode::InvalidArgument.into_errno(),
                    )?;
                    return Ok(());
                },
            },
            Err(_) => {
                ::nvx::pm::lookup_response(
                    destination,
                    ProcessIdentifier::from(u32::MAX),
                    ErrorCode::InvalidArgument.into_errno(),
                )?;
                return Ok(());
            },
        };

        // Check if process is the memory daemon.
        for (pid, pname) in self.processes.iter() {
            ::nvx::log!("looking up process (name={:?}, pname={:?})", name, pname);

            if pname == name {
                ::nvx::pm::lookup_response(destination, pid.clone(), 0)?;
                return Ok(());
            }
        }
        ::nvx::pm::lookup_response(
            destination,
            ProcessIdentifier::from(u32::MAX),
            ErrorCode::NoSuchEntry.into_errno(),
        )?;

        Ok(())
    }

    fn shutdown(&mut self) {
        ::nvx::log!("shutting down process manager daemon...");

        for (pid, pname) in self.processes.iter() {
            ::nvx::log!("shutting down process (pid={:?}, name={:?})", pid, pname);
            ::nvx::pm::shutdown(pid.clone(), 0).expect("failed to broadcast shutdown message");
        }

        // Wait for memory daemon to terminate.
        while self.processes.len() > 0 {
            match ::nvx::ipc::recv() {
                Ok(message) => match message.message_type {
                    MessageType::SchedulingEvent => {
                        // Deserialize process identifier.
                        let pid: ProcessIdentifier = ProcessIdentifier::from(u32::from_le_bytes(
                            message.payload[0..4].try_into().unwrap(),
                        ));

                        // Deserialize process status.
                        let status: i32 =
                            i32::from_le_bytes(message.payload[4..8].try_into().unwrap());

                        // De-register process.
                        if let Some(name) = self.processes.remove(&pid) {
                            ::nvx::log!(
                                "process terminated (name={:?}, pid={:?}, status={:?})",
                                name,
                                pid,
                                status
                            );
                        } else {
                            ::nvx::log!(
                                "unknown process terminated (pid={:?}, status={:?})",
                                pid,
                                status
                            );
                        }
                    },

                    // Ignore all other messages.
                    _ => {},
                },
                Err(e) => ::nvx::log!("failed to receive exception message (error={:?})", e),
            }
        }
    }
}

impl Drop for ProcessDaemon {
    fn drop(&mut self) {
        // Unsubscribe from scheduling events.
        ::nvx::log!("unsubscribing from scheduling events...");
        if let Err(e) = ::nvx::event::evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Unregister,
        ) {
            ::nvx::log!("failed to unsubscribe from scheduling events (error={:?})", e);
        }

        ::nvx::log!("shutting down process manager daemon...");
        if let Err(e) = ::nvx::pm::capctl(Capability::ProcessManagement, false) {
            ::nvx::log!("failed to release process management capabilities (error={:?})", e);
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    let mut procd: ProcessDaemon = match ProcessDaemon::init() {
        Ok(procd) => procd,
        Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
    };

    procd.run();
    procd.shutdown();

    Ok(())
}
