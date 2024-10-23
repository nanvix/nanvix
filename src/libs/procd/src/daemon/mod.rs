// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupMessage,
};
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
        ProcessIdentifier,
    },
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub struct ProcessDaemon {
    processes: BTreeMap<ProcessIdentifier, String>,
}

impl ProcessDaemon {
    // TODO: Change this, once we rename testd to initd.
    const INITD_NAME: &'static str = "testd";

    /// Initializes the process manager daemon.
    pub fn init() -> Result<Self, Error> {
        ::nvx::log!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::nvx::pm::getpid()?;
        assert_eq!(mypid, crate::PROCD, "process daemon has unexpected pid");

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
    pub fn run(&mut self) {
        loop {
            match ::nvx::ipc::recv() {
                Ok(message) => {
                    ::nvx::log!("received message from={:?}", { message.source });
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
        if let SystemMessageHeader::ProcessManagement = message.header {
            let message: ProcessManagementMessage =
                ProcessManagementMessage::from_bytes(message.payload)?;

            // Parse operation.
            match message.header {
                ProcessManagementMessageHeader::Signup => {
                    let message: SignupMessage = SignupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_signup(destionation, message)?;
                    ::nvx::ipc::send(&message)?;
                },
                ProcessManagementMessageHeader::Lookup => {
                    let message: LookupMessage = LookupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_lookup(destionation, message)?;
                    ::nvx::ipc::send(&message)?;
                },
                // Ignore all other messages.
                _ => {},
            }
        }

        Ok(())
    }

    // Handles a signup message.
    fn handle_signup(
        &mut self,
        destination: ProcessIdentifier,
        message: SignupMessage,
    ) -> Result<Message, Error> {
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
                    message::signup_response(destination, pid, 0)
                },
                Err(_) => message::signup_response(
                    destination,
                    pid,
                    ErrorCode::InvalidArgument.into_errno(),
                ),
            },
            Err(_) => {
                message::signup_response(destination, pid, ErrorCode::InvalidArgument.into_errno())
            },
        }
    }

    // Handles a lookup message.
    pub fn handle_lookup(
        &self,
        destination: ProcessIdentifier,
        message: LookupMessage,
    ) -> Result<Message, Error> {
        let name: &str = match CStr::from_bytes_until_nul(&message.name) {
            Ok(name) => match name.to_str() {
                Ok(s) => s,
                Err(_) => {
                    let message: Message = message::lookup_response(
                        destination,
                        ProcessIdentifier::from(u32::MAX),
                        ErrorCode::InvalidArgument.into_errno(),
                    )?;
                    return Ok(message);
                },
            },
            Err(_) => {
                let message: Message = message::lookup_response(
                    destination,
                    ProcessIdentifier::from(u32::MAX),
                    ErrorCode::InvalidArgument.into_errno(),
                )?;
                return Ok(message);
            },
        };

        // Check if process is the memory daemon.
        for (pid, pname) in self.processes.iter() {
            ::nvx::log!("looking up process (name={:?}, pname={:?})", name, pname);

            if pname == name {
                let message: Message = message::lookup_response(destination, *pid, 0)?;
                return Ok(message);
            }
        }
        let message: Message = message::lookup_response(
            destination,
            ProcessIdentifier::from(u32::MAX),
            ErrorCode::NoSuchEntry.into_errno(),
        )?;

        Ok(message)
    }

    pub fn shutdown(&mut self) {
        ::nvx::log!("shutting down process manager daemon...");

        for (pid, pname) in self.processes.iter() {
            ::nvx::log!("shutting down process (pid={:?}, name={:?})", pid, pname);
            let message: Message =
                message::shutdown_request(*pid, 0).expect("failed to broadcast shutdown message");
            ::nvx::ipc::send(&message).expect("failed to broadcast shutdown message");
        }

        // Wait for memory daemon to terminate.
        while !self.processes.is_empty() {
            match ::nvx::ipc::recv() {
                Ok(message) => {
                    if message.message_type == MessageType::SchedulingEvent {
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
                    }
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
